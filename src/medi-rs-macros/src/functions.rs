use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{
    Attribute, DeriveInput, Expr, FnArg, Ident, ItemFn, Path, Result as SynResult, Token, Type, Visibility, braced,
    bracketed, parse_macro_input, parse_quote,
};

// ---------------------------------------------------------------------------
// Derive macros (unchanged)
// ---------------------------------------------------------------------------

pub fn derive_medi_command_inner(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let return_type = match extract_return_type(&input.attrs) {
        Ok(Some(return_type)) => return_type,
        Ok(None) => parse_quote!(()),
        Err(error) => return error.to_compile_error().into(),
    };
    let error_type = match extract_error_type(&input.attrs) {
        Ok(Some(error_type)) => error_type,
        Ok(None) => parse_quote!(::core::convert::Infallible),
        Err(error) => return error.to_compile_error().into(),
    };

    let expanded = quote! {
        impl #impl_generics ::medi_rs::Command for #name #ty_generics #where_clause {
            type Response = #return_type;
        }

        impl #impl_generics ::medi_rs::StaticCommand for #name #ty_generics #where_clause {
            type Error = #error_type;
        }
    };

    TokenStream::from(expanded)
}

fn extract_return_type(attrs: &[Attribute]) -> syn::Result<Option<Type>> {
    let mut return_type = None;

    for attr in attrs {
        if !attr.path().is_ident("medi_command") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("return_type") {
                let value = meta.value()?;
                return_type = Some(value.parse::<Type>()?);
                Ok(())
            } else if meta.path.is_ident("error_type") {
                // Parsed separately by `extract_error_type`.
                let value = meta.value()?;
                let _: Type = value.parse()?;
                Ok(())
            } else {
                Err(meta
                    .error("unsupported medi_command attribute; expected `return_type = Type` or `error_type = Type`"))
            }
        })?;
    }

    Ok(return_type)
}

fn extract_error_type(attrs: &[Attribute]) -> syn::Result<Option<Type>> {
    let mut error_type = None;

    for attr in attrs {
        if !attr.path().is_ident("medi_command") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("error_type") {
                let value = meta.value()?;
                error_type = Some(value.parse::<Type>()?);
                Ok(())
            } else if meta.path.is_ident("return_type") {
                let value = meta.value()?;
                let _: Type = value.parse()?;
                Ok(())
            } else {
                Err(meta
                    .error("unsupported medi_command attribute; expected `return_type = Type` or `error_type = Type`"))
            }
        })?;
    }

    Ok(error_type)
}

// ---------------------------------------------------------------------------
// Composition support
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Static handler attribute
// ---------------------------------------------------------------------------

struct MediHandlerArgs {
    decorators: Vec<Path>,
}

impl Parse for MediHandlerArgs {
    fn parse(input: ParseStream<'_>) -> SynResult<Self> {
        if input.is_empty() {
            return Ok(Self { decorators: Vec::new() });
        }

        let key: Ident = input.parse()?;
        if key != "decorators" {
            return Err(syn::Error::new(key.span(), "expected `decorators`"));
        }
        input.parse::<Token![=]>()?;

        let content;
        bracketed!(content in input);
        let mut decorators = Vec::new();
        while !content.is_empty() {
            decorators.push(content.parse()?);
            if !content.is_empty() {
                content.parse::<Token![,]>()?;
            }
        }

        if !input.is_empty() {
            return Err(input.error("unexpected medi_handler attribute argument"));
        }

        Ok(Self { decorators })
    }
}

fn decorate_handler_call(
    decorators: &[Path],
    message: proc_macro2::TokenStream,
    handler_call: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    match decorators.split_first() {
        Some((decorator, remaining)) => {
            let next = decorate_handler_call(remaining, quote! { message }, handler_call);
            quote! { #decorator(#message, |message| async move { #next }).await }
        }
        None => quote! { #handler_call },
    }
}

pub fn medi_handler_inner(
    attribute: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let decorators = match syn::parse::<MediHandlerArgs>(attribute) {
        Ok(args) => args.decorators,
        Err(error) => return error.into_compile_error().into(),
    };
    let function = parse_macro_input!(input as ItemFn);
    let name = &function.sig.ident;
    let helper = format_ident!("__medi_handler_{name}");
    let mut arguments: Vec<&Type> = function
        .sig
        .inputs
        .iter()
        .filter_map(|argument| match argument {
            FnArg::Typed(argument) => Some(argument.ty.as_ref()),
            FnArg::Receiver(_) => None,
        })
        .collect();

    if arguments.is_empty() {
        return syn::Error::new_spanned(&function.sig, "a mediator handler requires a message parameter")
            .into_compile_error()
            .into();
    }

    let message = arguments.pop().expect("checked above");
    let (context, resources): (Option<&Type>, Vec<&Type>) = match arguments.first() {
        Some(Type::Reference(reference)) => (Some(reference.elem.as_ref()), arguments[1..].to_vec()),
        _ => (None, arguments),
    };
    let indexes: Vec<Ident> = (0..resources.len()).map(|index| format_ident!("I{index}")).collect();
    let call_arguments = resources.iter().zip(&indexes).map(|(resource, index)| {
        quote! { ::medi_rs::tlist::get::<#resource, #index, R>(resources) }
    });

    let handler_call = if context.is_some() {
        quote! { #name(mediator, #(#call_arguments,)* message).await }
    } else {
        quote! { #name(#(#call_arguments,)* message).await }
    };
    let helper_body = decorate_handler_call(&decorators, quote! { message }, &handler_call);
    let mediator_parameter = match context {
        Some(context) => quote! { mediator: &#context, },
        None => quote! { _mediator: &M, },
    };
    let helper_generics = if context.is_some() {
        quote! { <R, #(#indexes,)*> }
    } else {
        quote! { <M, R, #(#indexes,)*> }
    };
    let resource_bounds = resources.iter().zip(&indexes).map(|(resource, index)| {
        quote! { R: ::medi_rs::tlist::Get<#resource, #index>, }
    });
    let output = &function.sig.output;

    quote! {
        #function

        #[doc(hidden)]
        pub(crate) async fn #helper #helper_generics(
            #mediator_parameter
            resources: &R,
            message: #message,
        ) #output
        where
            #(#resource_bounds)*
        {
            #helper_body
        }
    }
    .into()
}

// ---------------------------------------------------------------------------
// Runtime task attribute
// ---------------------------------------------------------------------------

pub fn medi_task_inner(attribute: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    if !cfg!(any(feature = "tokio", feature = "wasm", feature = "embassy")) {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "`#[medi_task]` requires a medi-rs runtime feature",
        )
        .into_compile_error()
        .into();
    }
    if !attribute.is_empty() {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "`#[medi_task]` does not accept arguments",
        )
        .into_compile_error()
        .into();
    }

    let function = parse_macro_input!(input as ItemFn);
    let name = &function.sig.ident;
    let helper = format_ident!("__medi_task_{name}");
    let arguments: Vec<&Type> = function
        .sig
        .inputs
        .iter()
        .filter_map(|argument| match argument {
            FnArg::Typed(argument) => Some(argument.ty.as_ref()),
            FnArg::Receiver(_) => None,
        })
        .collect();
    let Some(Type::Reference(context)) = arguments.first() else {
        return syn::Error::new_spanned(
            &function.sig,
            "an Embassy task requires `&AppMediator` as its first parameter",
        )
        .into_compile_error()
        .into();
    };
    let resources = &arguments[1..];
    let indexes: Vec<Ident> = (0..resources.len()).map(|index| format_ident!("I{index}")).collect();
    let call_arguments = resources.iter().zip(&indexes).map(|(resource, index)| {
        quote! { ::medi_rs::tlist::get::<#resource, #index, R>(resources) }
    });
    let resource_bounds = resources.iter().zip(&indexes).map(|(resource, index)| {
        quote! { R: ::medi_rs::tlist::Get<#resource, #index>, }
    });
    let context = context.elem.as_ref();

    quote! {
        #function

        #[doc(hidden)]
        pub(crate) async fn #helper<R, #(#indexes,)*>(
            mediator: &#context,
            resources: &R,
        )
        where
            #(#resource_bounds)*
        {
            #name(mediator, #(#call_arguments,)*).await
        }
    }
    .into()
}

fn task_invoker_path(task: &syn::Path) -> syn::Path {
    let mut invoker = task.clone();
    let last = invoker
        .segments
        .last_mut()
        .expect("a syn::Path always has at least one segment");
    last.ident = format_ident!("__medi_task_{}", last.ident);
    last.arguments = syn::PathArguments::None;
    invoker
}

// ---------------------------------------------------------------------------
// Module-manifest composition spike
// ---------------------------------------------------------------------------

struct CommandManifest {
    request: Type,
    handler: syn::Path,
}

struct EventManifest {
    event: Type,
    handlers: Vec<syn::Path>,
}

struct ModuleManifest {
    commands: Vec<CommandManifest>,
    events: Vec<EventManifest>,
    resources: Vec<Type>,
    tasks: Vec<syn::Path>,
}

impl Parse for ModuleManifest {
    fn parse(input: ParseStream<'_>) -> SynResult<Self> {
        let mut manifest = Self {
            commands: Vec::new(),
            events: Vec::new(),
            resources: Vec::new(),
            tasks: Vec::new(),
        };

        while !input.is_empty() {
            let section: Ident = input.parse()?;
            let body;
            braced!(body in input);
            match section.to_string().as_str() {
                "commands" => manifest.commands.extend(parse_commands(&body)?),
                "events" => manifest.events.extend(parse_events(&body)?),
                "resources" => manifest.resources.extend(parse_resources(&body)?),
                "tasks" if cfg!(any(feature = "tokio", feature = "wasm", feature = "embassy")) => {
                    manifest.tasks.extend(parse_tasks(&body)?);
                }
                "tasks" => {
                    return Err(syn::Error::new(
                        section.span(),
                        "`tasks` requires a medi-rs runtime feature",
                    ));
                }
                _ => {
                    return Err(syn::Error::new(
                        section.span(),
                        "expected `commands`, `events`, `resources`, or `tasks`",
                    ));
                }
            }
        }

        Ok(manifest)
    }
}

fn parse_commands(body: ParseStream<'_>) -> SynResult<Vec<CommandManifest>> {
    let mut commands = Vec::new();
    while !body.is_empty() {
        let request = body.parse()?;
        body.parse::<Token![=>]>()?;
        let handler = body.parse()?;
        commands.push(CommandManifest { request, handler });
        if !body.is_empty() {
            body.parse::<Token![;]>()?;
        }
    }
    Ok(commands)
}

fn parse_events(body: ParseStream<'_>) -> SynResult<Vec<EventManifest>> {
    let mut events = Vec::new();
    while !body.is_empty() {
        let event = body.parse()?;
        body.parse::<Token![=>]>()?;
        let handlers_body;
        bracketed!(handlers_body in body);
        events.push(EventManifest {
            event,
            handlers: parse_event_handlers(&handlers_body)?,
        });
        if !body.is_empty() {
            body.parse::<Token![;]>()?;
        }
    }
    Ok(events)
}

fn parse_event_handlers(body: ParseStream<'_>) -> SynResult<Vec<syn::Path>> {
    let mut handlers = Vec::new();
    while !body.is_empty() {
        handlers.push(body.parse()?);
        if !body.is_empty() {
            body.parse::<Token![,]>()?;
        }
    }
    Ok(handlers)
}

fn parse_tasks(body: ParseStream<'_>) -> SynResult<Vec<syn::Path>> {
    let mut tasks = Vec::new();
    while !body.is_empty() {
        tasks.push(body.parse()?);
        if !body.is_empty() {
            body.parse::<Token![;]>()?;
        }
    }
    Ok(tasks)
}

fn parse_resources(body: ParseStream<'_>) -> SynResult<Vec<Type>> {
    let mut resources = Vec::new();
    while !body.is_empty() {
        resources.push(body.parse()?);
        if !body.is_empty() {
            body.parse::<Token![;]>()?;
        }
    }
    Ok(resources)
}

fn handler_invoker_path(handler: &syn::Path) -> syn::Path {
    let mut invoker = handler.clone();
    let last = invoker
        .segments
        .last_mut()
        .expect("a syn::Path always has at least one segment");
    last.ident = format_ident!("__medi_handler_{}", last.ident);
    last.arguments = syn::PathArguments::None;
    invoker
}

fn nested_tuple_type(resources: &[Type]) -> proc_macro2::TokenStream {
    resources.iter().rev().fold(quote! { () }, |tail, resource| {
        quote! { (#resource, #tail) }
    })
}

fn nested_tuple_value(resources: &[Ident]) -> proc_macro2::TokenStream {
    resources.iter().rev().fold(quote! { () }, |tail, resource| {
        quote! { (#resource, #tail) }
    })
}

struct MediModuleInput {
    manifest: Ident,
    module: ModuleManifest,
}

impl Parse for MediModuleInput {
    fn parse(input: ParseStream<'_>) -> SynResult<Self> {
        let manifest_key: Ident = input.parse()?;
        if manifest_key != "manifest" {
            return Err(syn::Error::new(manifest_key.span(), "expected `manifest`"));
        }
        let manifest: Ident = input.parse()?;
        input.parse::<Token![;]>()?;
        let module = input.parse()?;

        Ok(Self { manifest, module })
    }
}

/// Emit a local manifest macro which appends this module's declarations to the
/// composition accumulator. The accumulator is intentionally token based: a
/// later composition proc macro will parse the complete registration graph.
pub fn medi_module_inner(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as MediModuleInput);
    let manifest = input.manifest;
    let commands = input.module.commands.into_iter().map(|command| {
        let request = command.request;
        let handler = command.handler;
        quote! { #request => #handler; }
    });
    let events = input.module.events.into_iter().map(|event| {
        let event_type = event.event;
        let handlers = event.handlers;
        quote! { #event_type => [#(#handlers),*]; }
    });
    let resources = input.module.resources.into_iter().map(|resource| {
        quote! { #resource; }
    });
    let tasks: Vec<_> = input.module.tasks.into_iter().map(|task| quote! { #task; }).collect();
    // Do not emit an empty `tasks` section: task sections require a runtime,
    // while command-only manifests must remain usable without one.
    let tasks_section = (!tasks.is_empty()).then(|| quote! { tasks { #(#tasks)* } });

    quote! {
        macro_rules! #manifest {
            ($callback:path, {
                $vis:vis struct $name:ident;
                event_queue_capacity: $capacity:expr;
                event_workers: $workers:expr;
                modules: [$($modules:tt)*];
                decorators: [$($decorators:path),*];
                count: [$($count:tt)*];
                remaining: [$($remaining:ident),*];
            }) => {
                $callback! {
                    $vis struct $name;
                    event_queue_capacity: $capacity;
                    event_workers: $workers;
                    modules: [$($modules)* {
                        commands { #(#commands)* }
                        events { #(#events)* }
                        resources { #(#resources)* }
                        #tasks_section
                    },];
                    decorators: [$($decorators),*];
                    count: [$($count)* (),];
                    remaining: [$($remaining),*];
                }
            };
        }

        #[allow(unused_imports)]
        pub(crate) use #manifest;
    }
    .into()
}

struct CompositionMarkerInput {
    vis: Visibility,
    name: Ident,
    event_queue_capacity: Expr,
    event_workers: Expr,
    modules: Vec<ModuleManifest>,
    decorators: Vec<Path>,
    count: proc_macro2::TokenStream,
}

impl Parse for CompositionMarkerInput {
    fn parse(input: ParseStream<'_>) -> SynResult<Self> {
        let vis: Visibility = input.parse()?;
        input.parse::<Token![struct]>()?;
        let name: Ident = input.parse()?;
        input.parse::<Token![;]>()?;

        let capacity_key: Ident = input.parse()?;
        if capacity_key != "event_queue_capacity" {
            return Err(syn::Error::new(capacity_key.span(), "expected `event_queue_capacity`"));
        }
        input.parse::<Token![:]>()?;
        let event_queue_capacity = input.parse()?;
        input.parse::<Token![;]>()?;

        let workers_key: Ident = input.parse()?;
        if workers_key != "event_workers" {
            return Err(syn::Error::new(workers_key.span(), "expected `event_workers`"));
        }
        input.parse::<Token![:]>()?;
        let event_workers = input.parse()?;
        input.parse::<Token![;]>()?;

        let modules_key: Ident = input.parse()?;
        if modules_key != "modules" {
            return Err(syn::Error::new(modules_key.span(), "expected `modules`"));
        }
        input.parse::<Token![:]>()?;

        let modules;
        bracketed!(modules in input);
        let mut parsed_modules = Vec::new();
        while !modules.is_empty() {
            let manifest;
            braced!(manifest in modules);
            parsed_modules.push(manifest.parse()?);
            if !modules.is_empty() {
                modules.parse::<Token![,]>()?;
            }
        }
        input.parse::<Token![;]>()?;

        let decorators_key: Ident = input.parse()?;
        if decorators_key != "decorators" {
            return Err(syn::Error::new(decorators_key.span(), "expected `decorators`"));
        }
        input.parse::<Token![:]>()?;
        let decorators_body;
        bracketed!(decorators_body in input);
        let mut decorators = Vec::new();
        while !decorators_body.is_empty() {
            decorators.push(decorators_body.parse()?);
            if !decorators_body.is_empty() {
                decorators_body.parse::<Token![,]>()?;
            }
        }
        input.parse::<Token![;]>()?;

        let count_key: Ident = input.parse()?;
        if count_key != "count" {
            return Err(syn::Error::new(count_key.span(), "expected `count`"));
        }
        input.parse::<Token![:]>()?;
        let count;
        bracketed!(count in input);
        let count: proc_macro2::TokenStream = count.parse()?;
        input.parse::<Token![;]>()?;

        Ok(Self {
            vis,
            name,
            event_queue_capacity,
            event_workers,
            modules: parsed_modules,
            decorators,
            count,
        })
    }
}

fn combine_error(errors: &mut Option<syn::Error>, error: syn::Error) {
    if let Some(errors) = errors {
        errors.combine(error);
    } else {
        *errors = Some(error);
    }
}

fn validate_unique_registrations(modules: &[ModuleManifest]) -> SynResult<()> {
    let mut registrations = std::collections::HashMap::new();
    let mut resources = std::collections::HashMap::new();
    let mut errors = None;
    for module in modules {
        for command in &module.commands {
            let request = &command.request;
            let key = quote!(#request).to_string();
            if registrations.insert(key.clone(), request.span()).is_some() {
                combine_error(
                    &mut errors,
                    syn::Error::new(request.span(), format!("duplicate command registration for `{key}`")),
                );
            }
        }
        for resource in &module.resources {
            let key = quote!(#resource).to_string();
            if resources.insert(key.clone(), resource.span()).is_some() {
                combine_error(
                    &mut errors,
                    syn::Error::new(resource.span(), format!("duplicate resource registration for `{key}`")),
                );
            }
        }
    }
    errors.map_or(Ok(()), Err)
}

fn collect_event_routes(modules: &[ModuleManifest]) -> Vec<(Type, Vec<syn::Path>)> {
    let mut routes: Vec<(Type, Vec<syn::Path>)> = Vec::new();
    for event in modules.iter().flat_map(|module| &module.events) {
        let event_type = &event.event;
        let key = quote!(#event_type).to_string();
        if let Some((_, handlers)) = routes
            .iter_mut()
            .find(|(registered, _)| quote!(#registered).to_string() == key)
        {
            handlers.extend(event.handlers.iter().cloned());
        } else {
            routes.push((event.event.clone(), event.handlers.clone()));
        }
    }
    routes
}

/// Generated fragments for event-enabled mediators.
struct EventSupport {
    job: proc_macro2::TokenStream,
    field: proc_macro2::TokenStream,
    publish_routes: proc_macro2::TokenStream,
    publish_method: proc_macro2::TokenStream,
    worker: proc_macro2::TokenStream,
}

fn collect_tasks(modules: &[ModuleManifest]) -> Vec<syn::Path> {
    modules.iter().flat_map(|module| module.tasks.iter().cloned()).collect()
}

fn collect_resource_types(modules: &[ModuleManifest]) -> Vec<Type> {
    modules
        .iter()
        .flat_map(|module| module.resources.iter().cloned())
        .collect()
}

fn generate_constructor(
    resource_types: &[Type],
    resource_names: &[Ident],
    resource_values: &proc_macro2::TokenStream,
    has_events: bool,
    capacity: &Expr,
) -> proc_macro2::TokenStream {
    let configuration_check = quote! {
        assert!(#capacity > 0, "event_queue_capacity must be greater than zero");
    };
    match (resource_types.is_empty(), has_events) {
        (true, true) => quote! {
            /// Construct a mediator with no typed resources.
            pub fn new() -> Self {
                #configuration_check
                Self { resources: (), event_queue: ::medi_rs::EventQueue::new(Some(#capacity)) }
            }
        },
        (true, false) => quote! {
            /// Construct a mediator with no typed resources.
            pub const fn new() -> Self { Self { resources: () } }
        },
        (false, true) => quote! {
            /// Construct a mediator from its declared resource values.
            pub fn new((#(#resource_names,)*): (#(#resource_types,)*)) -> Self {
                #configuration_check
                Self { resources: #resource_values, event_queue: ::medi_rs::EventQueue::new(Some(#capacity)) }
            }
        },
        (false, false) => quote! {
            /// Construct a mediator from its declared resource values.
            pub fn new((#(#resource_names,)*): (#(#resource_types,)*)) -> Self {
                Self { resources: #resource_values }
            }
        },
    }
}

fn generate_task_workers(tasks: &[syn::Path], name: &Ident) -> Vec<proc_macro2::TokenStream> {
    tasks.iter().enumerate().map(|(index, task)| {
        let worker = format_ident!("medi_rs_task_{index}");
        let invoker = task_invoker_path(task);
        if cfg!(feature = "embassy") {
            quote! { #[::embassy_executor::task] async fn #worker(mediator: &'static #name) { #invoker(mediator, &mediator.resources).await; } }
        } else {
            quote! { async fn #worker(mediator: &'static #name) { #invoker(mediator, &mediator.resources).await; } }
        }
    }).collect()
}

fn generate_task_spawns(tasks: &[syn::Path]) -> Vec<proc_macro2::TokenStream> {
    tasks
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let worker = format_ident!("medi_rs_task_{index}");
            if cfg!(feature = "embassy") {
                quote! { spawner.spawn(#worker(self)).ok(); }
            } else {
                quote! { ::medi_rs::adapters::selected::spawn(#worker(self)); }
            }
        })
        .collect()
}

fn generate_command_routes(
    modules: &[ModuleManifest],
    name: &Ident,
    decorators: &[Path],
) -> Vec<proc_macro2::TokenStream> {
    modules.iter().flat_map(|module| module.commands.iter().map(|command| {
        let request = &command.request;
        let invoker = handler_invoker_path(&command.handler);
        let invocation = if decorators.is_empty() {
            quote! { #invoker(mediator, &mediator.resources, self).await }
        } else {
            decorate_handler_call(decorators, quote! { self }, &quote! { #invoker(mediator, &mediator.resources, message).await })
        };
        quote! {
            impl ::medi_rs::StaticSendCommand<#name> for #request {
                type Response = <#request as ::medi_rs::Command>::Response;
                type Error = <#request as ::medi_rs::StaticCommand>::Error;
                fn send(self, mediator: &#name) -> impl core::future::Future<Output = core::result::Result<Self::Response, Self::Error>> + Send {
                    async move { #invocation }
                }
            }
        }
    })).collect()
}

fn generate_event_dispatch_arms(
    routes: &[(Type, Vec<syn::Path>)],
    job: &Ident,
    decorators: &[Path],
) -> Vec<proc_macro2::TokenStream> {
    routes
        .iter()
        .enumerate()
        .map(|(index, (_, handlers))| {
            let variant = format_ident!("Event{index}");
            let calls = handlers.iter().map(|handler| {
                let invoker = handler_invoker_path(handler);
                let invocation = if decorators.is_empty() {
                    quote! { #invoker(mediator, &mediator.resources, event.clone()).await }
                } else {
                    decorate_handler_call(
                        decorators,
                        quote! { event.clone() },
                        &quote! { #invoker(mediator, &mediator.resources, message).await },
                    )
                };
                quote! { let _ = #invocation; }
            });
            quote! { #job::#variant(event) => { #(#calls)* } }
        })
        .collect()
}

fn generate_event_start(
    resource_tuple: &proc_macro2::TokenStream,
    worker: &Ident,
    task_spawns: &[proc_macro2::TokenStream],
) -> proc_macro2::TokenStream {
    if cfg!(feature = "embassy") {
        quote! {
            /// Start the generated Embassy event worker.
            pub fn start(&'static self, spawner: ::embassy_executor::Spawner) where #resource_tuple: Sync {
                assert!(Self::EVENT_WORKERS > 0, "event_workers must be greater than zero");
                for _ in 0..Self::EVENT_WORKERS { spawner.spawn(#worker(self)).ok(); }
                #(#task_spawns)*
            }
        }
    } else {
        quote! {
            /// Start the generated event worker and registered runtime tasks.
            pub fn start(&'static self) where #resource_tuple: Sync {
                assert!(Self::EVENT_WORKERS > 0, "event_workers must be greater than zero");
                for _ in 0..Self::EVENT_WORKERS { ::medi_rs::adapters::selected::spawn(#worker(self)); }
                #(#task_spawns)*
            }
        }
    }
}

fn generate_event_support(
    routes: &[(Type, Vec<syn::Path>)],
    name: &Ident,
    job: &Ident,
    capacity: &Expr,
    resource_tuple: &proc_macro2::TokenStream,
    decorators: &[Path],
    task_spawns: &[proc_macro2::TokenStream],
) -> EventSupport {
    let variants: Vec<_> = routes
        .iter()
        .enumerate()
        .map(|(index, (event, _))| {
            let variant = format_ident!("Event{index}");
            quote! { #variant(#event) }
        })
        .collect();
    let publish_routes: Vec<_> = routes
        .iter()
        .enumerate()
        .map(|(index, (event, _))| {
            let variant = format_ident!("Event{index}");
            quote! { impl ::medi_rs::StaticPublish<#name> for #event where #event: Clone + Send + 'static {
                fn publish(self, mediator: &#name) -> impl core::future::Future<Output = ::medi_rs::Result<()>> + Send {
                    ::medi_rs::EventQueue::publish(&mediator.event_queue, #job::#variant(self))
                }
            } }
        })
        .collect();
    let dispatch_arms = generate_event_dispatch_arms(routes, job, decorators);
    let worker = format_ident!("medi_rs_event_worker");
    let worker_loop = quote! { loop { match ::medi_rs::EventQueue::recv(&mediator.event_queue).await {
        Ok(event) => match event { #(#dispatch_arms)* }, Err(_) => break,
    } } };
    let worker_function = if cfg!(feature = "embassy") {
        quote! { #[allow(non_snake_case)] #[::embassy_executor::task] async fn #worker(mediator: &'static #name) { #worker_loop } }
    } else {
        quote! { #[allow(non_snake_case)] async fn #worker(mediator: &'static #name) { #worker_loop } }
    };
    let queue_type = if cfg!(feature = "embassy") {
        quote! { ::medi_rs::adapters::selected::EventQueue<#job, { #capacity }> }
    } else {
        quote! { ::medi_rs::adapters::selected::EventQueue<#job> }
    };
    let start = generate_event_start(resource_tuple, &worker, task_spawns);
    EventSupport {
        job: quote! { #[allow(non_camel_case_types)] enum #job { #(#variants,)* } },
        field: quote! { event_queue: #queue_type, },
        publish_routes: quote! { #(#publish_routes)* },
        publish_method: quote! { /// Enqueue an event for later worker dispatch.
        pub async fn publish<E>(&self, event: E) -> ::medi_rs::Result<()> where E: ::medi_rs::StaticPublish<Self> { event.publish(self).await } },
        worker: quote! { #worker_function impl #name { #start } },
    }
}

fn generate_task_only_start(
    has_events: bool,
    has_tasks: bool,
    name: &Ident,
    resource_tuple: &proc_macro2::TokenStream,
    task_spawns: &[proc_macro2::TokenStream],
) -> proc_macro2::TokenStream {
    if has_events || !has_tasks {
        return quote! {};
    }
    if cfg!(feature = "embassy") {
        quote! { impl #name { /// Start the registered Embassy tasks.
        pub fn start(&'static self, spawner: ::embassy_executor::Spawner) where #resource_tuple: Sync { #(#task_spawns)* } } }
    } else {
        quote! { impl #name { /// Start the registered runtime tasks.
        pub fn start(&'static self) where #resource_tuple: Sync { #(#task_spawns)* } } }
    }
}

/// Temporary composition endpoint. It validates the full registration graph and emits static dispatch code.
pub fn mediator_composition_marker_inner(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as CompositionMarkerInput);
    if let Err(error) = validate_unique_registrations(&input.modules) {
        return error.into_compile_error().into();
    }

    let event_routes = collect_event_routes(&input.modules);
    let has_events = !event_routes.is_empty();
    if has_events && !cfg!(any(feature = "tokio", feature = "wasm", feature = "embassy")) {
        return syn::Error::new(
            input.name.span(),
            "event mediators require the `tokio`, `wasm`, or `embassy` feature",
        )
        .into_compile_error()
        .into();
    }
    let resource_types = collect_resource_types(&input.modules);
    let resource_tuple = nested_tuple_type(&resource_types);
    let resource_names: Vec<_> = (0..resource_types.len())
        .map(|index| format_ident!("resource_{index}"))
        .collect();
    let resource_values = nested_tuple_value(&resource_names);
    let tasks = collect_tasks(&input.modules);
    let task_workers = generate_task_workers(&tasks, &input.name);
    let task_spawns = generate_task_spawns(&tasks);
    let command_routes = generate_command_routes(&input.modules, &input.name, &input.decorators);
    let constructor = generate_constructor(
        &resource_types,
        &resource_names,
        &resource_values,
        has_events,
        &input.event_queue_capacity,
    );
    let job_name = format_ident!("{}EventJob", input.name);
    let event_support = has_events.then(|| {
        generate_event_support(
            &event_routes,
            &input.name,
            &job_name,
            &input.event_queue_capacity,
            &resource_tuple,
            &input.decorators,
            &task_spawns,
        )
    });
    let event_job = event_support
        .as_ref()
        .map_or_else(|| quote! {}, |support| support.job.clone());
    let event_field = event_support
        .as_ref()
        .map_or_else(|| quote! {}, |support| support.field.clone());
    let publish_routes = event_support
        .as_ref()
        .map_or_else(|| quote! {}, |support| support.publish_routes.clone());
    let publish_method = event_support
        .as_ref()
        .map_or_else(|| quote! {}, |support| support.publish_method.clone());
    let event_worker = event_support
        .as_ref()
        .map_or_else(|| quote! {}, |support| support.worker.clone());
    let task_only_start = generate_task_only_start(
        has_events,
        !tasks.is_empty(),
        &input.name,
        &resource_tuple,
        &task_spawns,
    );
    let vis = input.vis;
    let name = input.name;
    let capacity = input.event_queue_capacity;
    let workers = input.event_workers;
    let count = input.count;
    quote! {
        #event_job
        #vis struct #name { resources: #resource_tuple, #event_field }
        impl #name {
            #constructor
            /// Configured capacity for the generated event queue.
            pub const EVENT_QUEUE_CAPACITY: usize = #capacity;
            /// Number of event worker tasks started by [`Self::start`].
            pub const EVENT_WORKERS: usize = #workers;
            /// Number of manifests included in this composition.
            pub const MODULE_COUNT: usize = <[()]>::len(&[#count]);
            /// Send a command through its macro-generated static route.
            pub async fn send<C>(&self, command: C) -> core::result::Result<C::Response, C::Error> where C: ::medi_rs::StaticSendCommand<Self> { command.send(self).await }
            #publish_method
        }
        #(#command_routes)* #publish_routes #event_worker #(#task_workers)* #task_only_start
    }.into()
}

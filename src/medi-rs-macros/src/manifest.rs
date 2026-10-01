//! `medi_module!` manifest parsing and expansion.

use proc_macro2::{Group, Punct, Spacing, TokenStream, TokenTree};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Ident, Result as SynResult, Token, Type, braced, bracketed, parse_macro_input};

pub(crate) struct CommandManifest {
    pub(crate) request: Type,
    pub(crate) handler: syn::Path,
}

pub(crate) struct EventManifest {
    pub(crate) event: Type,
    pub(crate) handlers: Vec<syn::Path>,
}

pub(crate) struct ResourceManifest {
    pub(crate) name: Option<Ident>,
    pub(crate) resource_type: Type,
}

pub(crate) struct ModuleManifest {
    pub(crate) commands: Vec<CommandManifest>,
    pub(crate) streams: Vec<CommandManifest>,
    pub(crate) events: Vec<EventManifest>,
    pub(crate) resources: Vec<ResourceManifest>,
    pub(crate) tasks: Vec<syn::Path>,
    pub(crate) startup: Vec<syn::Path>,
    pub(crate) shutdown: Vec<syn::Path>,
}

impl Parse for ModuleManifest {
    fn parse(input: ParseStream<'_>) -> SynResult<Self> {
        let mut manifest = Self {
            commands: Vec::new(),
            streams: Vec::new(),
            events: Vec::new(),
            resources: Vec::new(),
            tasks: Vec::new(),
            startup: Vec::new(),
            shutdown: Vec::new(),
        };

        while !input.is_empty() {
            let section: Ident = input.parse()?;
            let body;
            braced!(body in input);
            match section.to_string().as_str() {
                "commands" => manifest.commands.extend(parse_commands(&body)?),
                "streams" => manifest.streams.extend(parse_commands(&body)?),
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
                "startup" if cfg!(any(feature = "tokio", feature = "wasm", feature = "embassy")) => {
                    manifest.startup.extend(parse_tasks(&body)?);
                }
                "shutdown" if cfg!(any(feature = "tokio", feature = "wasm", feature = "embassy")) => {
                    manifest.shutdown.extend(parse_tasks(&body)?);
                }
                "startup" | "shutdown" => {
                    return Err(syn::Error::new(
                        section.span(),
                        "lifecycle hooks require a medi-rs runtime feature",
                    ));
                }
                _ => {
                    return Err(syn::Error::new(
                        section.span(),
                        "expected `commands`, `streams`, `events`, `resources`, `tasks`, `startup`, or `shutdown`",
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

fn parse_resources(body: ParseStream<'_>) -> SynResult<Vec<ResourceManifest>> {
    let mut resources = Vec::new();
    while !body.is_empty() {
        let name: Ident = body.parse()?;
        if !body.peek(Token![:]) {
            return Err(syn::Error::new(
                name.span(),
                "resource declarations must use `name: Type`",
            ));
        }
        body.parse::<Token![:]>()?;
        resources.push(ResourceManifest {
            name: Some(name),
            resource_type: body.parse()?,
        });
        if !body.is_empty() {
            body.parse::<Token![;]>()?;
        }
    }
    Ok(resources)
}

pub(crate) fn handler_invoker_path(handler: &syn::Path) -> syn::Path {
    let mut invoker = handler.clone();
    let last = invoker
        .segments
        .last_mut()
        .expect("a syn::Path always has at least one segment");
    last.ident = format_ident!("__medi_handler_{}", last.ident);
    last.arguments = syn::PathArguments::None;
    invoker
}

struct MediModuleInput {
    exported: bool,
    manifest: Ident,
    module: ModuleManifest,
}

impl Parse for MediModuleInput {
    fn parse(input: ParseStream<'_>) -> SynResult<Self> {
        let exported = input.peek(Token![pub]);
        if exported {
            input.parse::<Token![pub]>()?;
        }
        let manifest_key: Ident = input.parse()?;
        if manifest_key != "manifest" {
            return Err(syn::Error::new(
                manifest_key.span(),
                "expected `manifest` or `pub manifest`",
            ));
        }
        let manifest: Ident = input.parse()?;
        input.parse::<Token![;]>()?;
        let module = input.parse()?;

        Ok(Self {
            exported,
            manifest,
            module,
        })
    }
}

/// Replace `crate` in user paths with macro_rules' `$crate`, so a public
/// manifest keeps referring to its defining crate when expanded by a consumer.
fn externalize_crate_paths(tokens: TokenStream) -> TokenStream {
    tokens
        .into_iter()
        .flat_map(|token| match token {
            TokenTree::Ident(ident) if ident == "crate" => vec![
                TokenTree::Punct(Punct::new('$', Spacing::Alone)),
                TokenTree::Ident(ident),
            ],
            TokenTree::Group(group) => {
                let mut rewritten = Group::new(group.delimiter(), externalize_crate_paths(group.stream()));
                rewritten.set_span(group.span());
                vec![TokenTree::Group(rewritten)]
            }
            token => vec![token],
        })
        .collect()
}

/// Emit a local manifest macro which appends this module's declarations to the
/// composition accumulator. The accumulator is intentionally token based: a
/// later composition proc macro will parse the complete registration graph.
pub fn medi_module_inner(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as MediModuleInput);
    let exported = input.exported;
    let manifest = input.manifest;
    let rewrite = |tokens| {
        if exported {
            externalize_crate_paths(tokens)
        } else {
            tokens
        }
    };
    let commands = input.module.commands.into_iter().map(|command| {
        let CommandManifest { request, handler } = command;
        let request = rewrite(quote! { #request });
        let handler = rewrite(quote! { #handler });
        quote! { #request => #handler; }
    });
    let streams = input.module.streams.into_iter().map(|stream| {
        let CommandManifest { request, handler } = stream;
        let request = rewrite(quote! { #request });
        let handler = rewrite(quote! { #handler });
        quote! { #request => #handler; }
    });
    let events = input.module.events.into_iter().map(|event| {
        let EventManifest { event, handlers } = event;
        let event_type = rewrite(quote! { #event });
        let handlers: Vec<_> = handlers
            .into_iter()
            .map(|handler| rewrite(quote! { #handler }))
            .collect();
        quote! { #event_type => [#(#handlers),*]; }
    });
    let resources = input.module.resources.into_iter().map(|resource| {
        let ResourceManifest { name, resource_type } = resource;
        let resource_type = rewrite(quote! { #resource_type });
        if let Some(name) = name {
            quote! { #name: #resource_type; }
        } else {
            quote! { #resource_type; }
        }
    });
    let tasks: Vec<_> = input
        .module
        .tasks
        .into_iter()
        .map(|task| rewrite(quote! { #task; }))
        .collect();
    let startup: Vec<_> = input
        .module
        .startup
        .into_iter()
        .map(|hook| rewrite(quote! { #hook; }))
        .collect();
    let shutdown: Vec<_> = input
        .module
        .shutdown
        .into_iter()
        .map(|hook| rewrite(quote! { #hook; }))
        .collect();
    // Do not emit empty runtime-only sections so command-only manifests remain usable without a runtime.
    let tasks_section = (!tasks.is_empty()).then(|| quote! { tasks { #(#tasks)* } });
    let startup_section = (!startup.is_empty()).then(|| quote! { startup { #(#startup)* } });
    let shutdown_section = (!shutdown.is_empty()).then(|| quote! { shutdown { #(#shutdown)* } });

    let export_attribute = exported.then(|| quote! { #[macro_export] });
    let manifest_use = (!exported).then(|| {
        quote! {
            #[allow(unused_imports)]
            pub(crate) use #manifest;
        }
    });

    quote! {
        #export_attribute
        macro_rules! #manifest {
            ($callback:path, {
                $vis:vis struct $name:ident;
                event_queue_capacity: $capacity:expr;
                event_workers: $workers:expr;
                modules: [$($modules:tt)*];
                decorators: [$($decorators:path),*];
                event_failure_reporter: [$($reporter:ty)?];
                count: [$($count:tt)*];
                remaining: [$($remaining:ident),*];
            }) => {
                $callback! {
                    $vis struct $name;
                    event_queue_capacity: $capacity;
                    event_workers: $workers;
                    modules: [$($modules)* {
                        commands { #(#commands)* }
                        streams { #(#streams)* }
                        events { #(#events)* }
                        resources { #(#resources)* }
                        #tasks_section
                        #startup_section
                        #shutdown_section
                    },];
                    decorators: [$($decorators),*];
                    event_failure_reporter: [$($reporter)?];
                    count: [$($count)* (),];
                    remaining: [$($remaining),*];
                }
            };
        }

        #manifest_use
    }
    .into()
}

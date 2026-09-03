//! `medi_module!` manifest parsing and expansion.

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

pub(crate) struct ModuleManifest {
    pub(crate) commands: Vec<CommandManifest>,
    pub(crate) events: Vec<EventManifest>,
    pub(crate) resources: Vec<Type>,
    pub(crate) tasks: Vec<syn::Path>,
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

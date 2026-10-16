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

#[derive(Clone)]
pub(crate) struct ActorModuleManifest {
    pub(crate) module_type: Type,
    pub(crate) mailbox_capacity: syn::Expr,
}

pub(crate) struct ModuleManifest {
    pub(crate) actor: Option<ActorModuleManifest>,
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
        let mut module_type = None;
        let mut mailbox_capacity = None;
        let mut manifest = Self {
            actor: None,
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
            match section.to_string().as_str() {
                "module" => {
                    if module_type.is_some() {
                        return Err(syn::Error::new(section.span(), "duplicate `module` declaration"));
                    }
                    module_type = Some(input.parse()?);
                    input.parse::<Token![;]>()?;
                    continue;
                }
                "mailbox_capacity" => {
                    if mailbox_capacity.is_some() {
                        return Err(syn::Error::new(
                            section.span(),
                            "duplicate `mailbox_capacity` declaration",
                        ));
                    }
                    input.parse::<Token![:]>()?;
                    mailbox_capacity = Some(input.parse()?);
                    input.parse::<Token![;]>()?;
                    continue;
                }
                _ => {}
            }
            let body;
            braced!(body in input);
            match section.to_string().as_str() {
                "commands" => manifest.commands.extend(parse_commands(&body)?),
                "events" => manifest.events.extend(parse_events(&body)?),
                _ => {
                    return Err(syn::Error::new(
                        section.span(),
                        "expected `commands` or `events`; streams, resources, tasks, and lifecycle hooks were removed in v4",
                    ));
                }
            }
        }

        manifest.actor = match (module_type, mailbox_capacity) {
            (Some(module_type), Some(mailbox_capacity)) => Some(ActorModuleManifest {
                module_type,
                mailbox_capacity,
            }),
            (None, None) => {
                return Err(syn::Error::new(
                    input.span(),
                    "v4 modules require `module Type; mailbox_capacity: N;`",
                ));
            }
            (Some(module_type), None) => {
                return Err(syn::Error::new_spanned(
                    module_type,
                    "actor modules require `mailbox_capacity: N;`",
                ));
            }
            (None, Some(mailbox_capacity)) => {
                return Err(syn::Error::new_spanned(
                    mailbox_capacity,
                    "`mailbox_capacity` requires a `module Type;` declaration",
                ));
            }
        };
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
    let actor = input.module.actor.map(|actor| {
        let ActorModuleManifest {
            module_type,
            mailbox_capacity,
        } = actor;
        let module_type = rewrite(quote! { #module_type });
        quote! { module #module_type; mailbox_capacity: #mailbox_capacity; }
    });
    let commands = input.module.commands.into_iter().map(|command| {
        let CommandManifest { request, handler } = command;
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
                remaining: [$($remaining:tt)*];
            }) => {
                $callback! {
                    $vis struct $name;
                    event_queue_capacity: $capacity;
                    event_workers: $workers;
                    modules: [$($modules)* {
                        #actor
                        commands { #(#commands)* }
                        events { #(#events)* }
                    },];
                    decorators: [$($decorators),*];
                    event_failure_reporter: [$($reporter)?];
                    count: [$($count)* (),];
                    remaining: [$($remaining)*];
                }
            };
        }

        #manifest_use
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::ModuleManifest;
    use syn::parse_str;

    #[test]
    fn parses_actor_module_and_mailbox_capacity() {
        let manifest = parse_str::<ModuleManifest>(
            "module ButtonModule; mailbox_capacity: 8; commands { EnableButton => ButtonModule::enable; }",
        )
        .expect("actor module manifest should parse");
        let actor = manifest.actor.expect("actor declaration should be retained");
        let module_type = actor.module_type;
        let mailbox_capacity = actor.mailbox_capacity;
        assert_eq!(quote::quote!(#module_type).to_string(), "ButtonModule");
        assert_eq!(quote::quote!(#mailbox_capacity).to_string(), "8");
        assert_eq!(manifest.commands.len(), 1);
    }

    #[test]
    fn requires_capacity_when_module_type_is_declared() {
        let error = match parse_str::<ModuleManifest>("module ButtonModule;") {
            Ok(_) => panic!("module declaration without capacity must fail"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("mailbox_capacity"));
    }
}

//! `#[medi_task]` parsing and expansion.

use quote::{format_ident, quote};
use syn::{FnArg, Ident, ItemFn, Type, parse_macro_input, parse_quote};

struct ContextSpec {
    request: Type,
    kind: ContextKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ContextKind {
    Command,
    Event,
    Stream,
}

impl ContextKind {
    fn marker(self, route: &Type) -> proc_macro2::TokenStream {
        match self {
            Self::Command => quote! { ::medi_rs::MediCommand<#route> },
            Self::Event => quote! { ::medi_rs::MediEvent<#route> },
            Self::Stream => quote! { ::medi_rs::MediStream<#route> },
        }
    }

    fn bound(self, route: &Type, mediator: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        match self {
            Self::Command => quote! { #route: ::medi_rs::StaticSendCommand<#mediator> },
            Self::Event => quote! { #route: ::medi_rs::StaticPublish<#mediator> },
            Self::Stream => quote! { #route: ::medi_rs::StaticStream<#mediator> },
        }
    }
}

struct TaskParameters {
    mediator: Option<Type>,
    contexts: Vec<ContextSpec>,
    signal: bool,
    resources: Vec<Type>,
}

fn runtime_is_enabled() -> bool {
    cfg!(any(feature = "tokio", feature = "wasm", feature = "embassy"))
}

fn is_shutdown_signal(ty: &Type) -> bool {
    matches!(ty, Type::Reference(reference) if matches!(reference.elem.as_ref(), Type::Path(path) if path.path.segments.last().is_some_and(|segment| segment.ident == "ShutdownSignal")))
}

fn is_mediator(ty: &Type) -> bool {
    matches!(ty, Type::Reference(reference) if matches!(reference.elem.as_ref(), Type::Path(path) if path.path.segments.last().is_some_and(|segment| segment.ident.to_string().ends_with("Mediator"))))
}

fn context_spec(ty: &Type) -> Option<ContextSpec> {
    let Type::Path(path) = ty else { return None };
    let segment = path.path.segments.last()?;
    if segment.ident != "MediCtx" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    let request = args.args.iter().find_map(|arg| match arg {
        syn::GenericArgument::Type(ty) => Some(ty.clone()),
        _ => None,
    })?;
    let Type::Path(request_path) = &request else {
        return None;
    };
    let wrapper = request_path.path.segments.last()?;
    let kind = match wrapper.ident.to_string().as_str() {
        "MediCommand" => ContextKind::Command,
        "MediEvent" => ContextKind::Event,
        "MediStream" => ContextKind::Stream,
        _ => return None,
    };
    let syn::PathArguments::AngleBracketed(args) = &wrapper.arguments else {
        return None;
    };
    let request = args.args.iter().find_map(|arg| match arg {
        syn::GenericArgument::Type(ty) => Some(ty.clone()),
        _ => None,
    })?;
    Some(ContextSpec { request, kind })
}

fn add_context_bounds(function: &mut ItemFn, contexts: &[ContextSpec], mediator: &proc_macro2::TokenStream) {
    for context in contexts {
        let predicate = context.kind.bound(&context.request, mediator);
        function
            .sig
            .generics
            .make_where_clause()
            .predicates
            .push(parse_quote!(#predicate));
    }
}

fn replace_context_parameters(function: &mut ItemFn, mediator: &proc_macro2::TokenStream) {
    for argument in &mut function.sig.inputs {
        let FnArg::Typed(argument) = argument else { continue };
        if let Some(context) = context_spec(&argument.ty) {
            let marker = context.kind.marker(&context.request);
            let context_type: Type = parse_quote!(::medi_rs::MediCtx<'_, #marker, #mediator>);
            *argument.ty = context_type;
        }
    }
}

fn typed_arguments(function: &ItemFn) -> Vec<Type> {
    function
        .sig
        .inputs
        .iter()
        .filter_map(|argument| match argument {
            FnArg::Typed(argument) => Some((*argument.ty).clone()),
            FnArg::Receiver(_) => None,
        })
        .collect()
}

fn split_parameters(mut arguments: Vec<Type>) -> TaskParameters {
    let mediator = arguments.first().filter(|ty| is_mediator(ty)).cloned();
    if mediator.is_some() {
        arguments.remove(0);
    }
    let mut contexts = Vec::new();
    while arguments.first().is_some_and(|ty| context_spec(ty).is_some()) {
        contexts.push(context_spec(&arguments.remove(0)).expect("checked above"));
    }
    let (signal, resources) = match arguments.first() {
        Some(signal) if is_shutdown_signal(signal) => (true, arguments[1..].to_vec()),
        _ => (false, arguments),
    };
    TaskParameters {
        mediator,
        contexts,
        signal,
        resources,
    }
}

pub fn medi_task_inner(attribute: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    if !runtime_is_enabled() {
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

    let mut function = parse_macro_input!(input as ItemFn);
    let name = function.sig.ident.clone();
    let helper = format_ident!("__medi_task_{name}");
    let parameters = split_parameters(typed_arguments(&function));
    let indexes: Vec<Ident> = (0..parameters.resources.len())
        .map(|index| format_ident!("I{index}"))
        .collect();
    let call_arguments = parameters
        .resources
        .iter()
        .zip(&indexes)
        .map(|(resource, index)| match resource {
            Type::Reference(reference) => {
                let resource = &reference.elem;
                quote! { ::medi_rs::tlist::get_ref::<#resource, #index, R>(resources), }
            }
            _ => quote! { ::medi_rs::tlist::get::<#resource, #index, R>(resources), },
        });
    let resource_bounds = parameters
        .resources
        .iter()
        .zip(&indexes)
        .map(|(resource, index)| match resource {
            Type::Reference(reference) => {
                let resource = &reference.elem;
                quote! { R: ::medi_rs::tlist::GetRef<#resource, #index>, }
            }
            _ => quote! { R: ::medi_rs::tlist::Get<#resource, #index>, },
        });

    let hidden_mediator = parameters.mediator.is_none();
    let mediator_type = if hidden_mediator && !parameters.contexts.is_empty() {
        crate::common::fresh_type_parameter(&function.sig.generics, "__MediRsMediator")
    } else {
        format_ident!("__MediRsMediator")
    };
    if !parameters.contexts.is_empty() {
        if hidden_mediator {
            function.sig.generics.params.push(parse_quote!(#mediator_type));
        }
        let route_mediator = parameters.mediator.as_ref().map_or_else(
            || quote! { #mediator_type },
            |ty| {
                let Type::Reference(reference) = ty else { unreachable!() };
                let elem = &reference.elem;
                quote! { #elem }
            },
        );
        add_context_bounds(&mut function, &parameters.contexts, &route_mediator);
        replace_context_parameters(&mut function, &route_mediator);
    }

    let mediator_parameter = parameters
        .mediator
        .as_ref()
        .map_or_else(|| quote! { mediator: &#mediator_type, }, |ty| quote! { mediator: #ty, });
    let helper_generics = if parameters.mediator.is_some() {
        quote! { <R, #(#indexes,)*> }
    } else {
        quote! { <#mediator_type, R, #(#indexes,)*> }
    };
    let route_mediator = if let Some(ty) = &parameters.mediator {
        let Type::Reference(reference) = ty else { unreachable!() };
        let elem = &reference.elem;
        quote! { #elem }
    } else {
        quote! { #mediator_type }
    };
    let context_arguments = parameters
        .contexts
        .iter()
        .map(|_| quote! { ::medi_rs::MediCtx::new(mediator), });
    let mediator_argument = parameters
        .mediator
        .as_ref()
        .map_or_else(|| quote! {}, |_| quote! { mediator, });
    let signal_argument = if parameters.signal {
        quote! { signal, }
    } else {
        proc_macro2::TokenStream::new()
    };
    let task_call = quote! {
        #name(#mediator_argument #(#context_arguments)* #signal_argument #(#call_arguments)*).await
    };
    let helper_route_bounds = parameters.contexts.iter().map(|context| {
        let bound = context.kind.bound(&context.request, &route_mediator);
        quote! { #bound, }
    });

    quote! {
        #function

        #[doc(hidden)]
        pub async fn #helper #helper_generics(
            #mediator_parameter
            resources: &R,
            signal: &'static ::medi_rs::ShutdownSignal,
        )
        where
            #(#resource_bounds)*
            #(#helper_route_bounds)*
        {
            #task_call
        }
    }
    .into()
}

pub(crate) fn task_invoker_path(task: &syn::Path) -> syn::Path {
    let mut invoker = task.clone();
    let last = invoker
        .segments
        .last_mut()
        .expect("a syn::Path always has at least one segment");
    last.ident = format_ident!("__medi_task_{}", last.ident);
    last.arguments = syn::PathArguments::None;
    invoker
}

#[cfg(test)]
mod tests {
    use super::{ContextKind, context_spec};
    use quote::ToTokens;
    use syn::parse_quote;

    #[test]
    fn parses_all_mediator_context_markers() {
        for (ty, expected, kind) in [
            (
                parse_quote!(MediCtx<MediCommand<Refresh>>),
                "Refresh",
                ContextKind::Command,
            ),
            (parse_quote!(MediCtx<MediEvent<Changed>>), "Changed", ContextKind::Event),
            (parse_quote!(MediCtx<MediStream<Search>>), "Search", ContextKind::Stream),
        ] {
            let spec = context_spec(&ty).expect("valid context marker");
            assert_eq!(spec.request.to_token_stream().to_string(), expected);
            assert!(matches!(
                (spec.kind, kind),
                (ContextKind::Command, ContextKind::Command)
                    | (ContextKind::Event, ContextKind::Event)
                    | (ContextKind::Stream, ContextKind::Stream)
            ));
        }
    }

    #[test]
    fn rejects_unmarked_or_malformed_context_types() {
        assert!(context_spec(&parse_quote!(Repository)).is_none());
        assert!(context_spec(&parse_quote!(MediCtx<Unknown<Command>>)).is_none());
        assert!(context_spec(&parse_quote!(MediCtx<MediCommand>)).is_none());
    }
}

//! `#[medi_handler]` parsing and expansion.

use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{FnArg, Ident, ItemFn, Path, Result as SynResult, Token, Type, bracketed, parse_macro_input, parse_quote};

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

pub(crate) fn decorate_handler_call(
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

/// Parameters shared by generated handler invokers: an optional mediator
/// context followed by injected resources.
pub(crate) struct InjectedParameters<'a> {
    pub(crate) context: Option<&'a Type>,
    pub(crate) resources: Vec<&'a Type>,
    pub(crate) indexes: Vec<Ident>,
}

impl<'a> InjectedParameters<'a> {
    /// Split the parameters preceding the handler's message (and, for stream
    /// handlers, its sender) into an optional mediator context and resources.
    pub(crate) fn parse(arguments: Vec<&'a Type>, detect_mediator: bool) -> SynResult<Self> {
        let (context, resources): (Option<&Type>, Vec<&Type>) = match arguments.first() {
            _ if !detect_mediator => (None, arguments),
            Some(Type::Reference(reference)) if reference.mutability.is_some() => {
                return Err(syn::Error::new_spanned(
                    reference,
                    "a mutable mediator reference is not supported; use `&AppMediator`",
                ));
            }
            Some(Type::Reference(reference)) => (Some(reference.elem.as_ref()), arguments[1..].to_vec()),
            _ => (None, arguments),
        };
        if let Some(reference) = resources.iter().find_map(|resource| match resource {
            Type::Reference(reference) if reference.mutability.is_some() => Some(reference),
            _ => None,
        }) {
            return Err(syn::Error::new_spanned(
                reference,
                "mutable resource references are not supported; use a synchronization primitive or an owner task",
            ));
        }
        let indexes = (0..resources.len()).map(|index| format_ident!("I{index}")).collect();
        Ok(Self {
            context,
            resources,
            indexes,
        })
    }

    /// Call the handler with the optional mediator and resolved resources
    /// followed by `trailing` arguments.
    pub(crate) fn handler_call(&self, name: &Ident, trailing: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        let call_arguments = self
            .resources
            .iter()
            .zip(&self.indexes)
            .map(|(resource, index)| match resource {
                Type::Reference(reference) => {
                    let resource = &reference.elem;
                    quote! { ::medi_rs::tlist::get_ref::<#resource, #index, R>(resources) }
                }
                _ => quote! { ::medi_rs::tlist::get::<#resource, #index, R>(resources) },
            });
        if self.context.is_some() {
            quote! { #name(mediator, #(#call_arguments,)* #trailing).await }
        } else {
            quote! { #name(#(#call_arguments,)* #trailing).await }
        }
    }

    pub(crate) fn mediator_parameter(&self) -> proc_macro2::TokenStream {
        match self.context {
            Some(context) => quote! { mediator: &#context, },
            None => quote! { _mediator: &M, },
        }
    }

    /// Invoker generics, preceded by `lifetimes` declared on the handler.
    pub(crate) fn helper_generics<'l>(
        &self,
        lifetimes: impl Iterator<Item = &'l syn::LifetimeParam>,
    ) -> proc_macro2::TokenStream {
        let indexes = &self.indexes;
        if self.context.is_some() {
            quote! { <#(#lifetimes,)* R, #(#indexes,)*> }
        } else {
            quote! { <#(#lifetimes,)* M, R, #(#indexes,)*> }
        }
    }

    pub(crate) fn resource_bounds(&self) -> Vec<proc_macro2::TokenStream> {
        self.resources
            .iter()
            .zip(&self.indexes)
            .map(|(resource, index)| match resource {
                Type::Reference(reference) => {
                    let resource = &reference.elem;
                    quote! { R: ::medi_rs::tlist::GetRef<#resource, #index>, }
                }
                _ => quote! { R: ::medi_rs::tlist::Get<#resource, #index>, },
            })
            .collect()
    }
}

/// The typed parameter list of a handler function.
#[derive(Debug, Eq, PartialEq)]
enum MediCtxKind {
    Command,
    Event,
    Stream,
}

type Context = (usize, Type, MediCtxKind);

impl MediCtxKind {
    fn marker(&self, route: &Type) -> proc_macro2::TokenStream {
        match self {
            Self::Command => quote! { ::medi_rs::MediCommand<#route> },
            Self::Event => quote! { ::medi_rs::MediEvent<#route> },
            Self::Stream => quote! { ::medi_rs::MediStream<#route> },
        }
    }

    fn bound(&self, route: &Type, mediator: &Ident) -> proc_macro2::TokenStream {
        match self {
            Self::Command => quote! { #route: ::medi_rs::StaticSendCommand<#mediator> },
            Self::Event => quote! { #route: ::medi_rs::StaticPublish<#mediator> },
            Self::Stream => quote! { #route: ::medi_rs::StaticStream<#mediator> },
        }
    }
}

fn medi_ctx_command(ty: &Type) -> Option<(Type, MediCtxKind)> {
    let Type::Path(path) = ty else { return None };
    let segment = path.path.segments.last()?;
    if segment.ident != "MediCtx" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    let mut route = arguments.args.iter().find_map(|argument| match argument {
        syn::GenericArgument::Type(command) => Some(command.clone()),
        _ => None,
    })?;
    let Type::Path(path) = &route else { return None };
    let segment = path.path.segments.last()?;
    let marker = segment.ident.to_string();
    let kind = match marker.as_str() {
        "MediCommand" => MediCtxKind::Command,
        "MediEvent" => MediCtxKind::Event,
        "MediStream" => MediCtxKind::Stream,
        _ => return None,
    };
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    route = arguments.args.iter().find_map(|argument| match argument {
        syn::GenericArgument::Type(route) => Some(route.clone()),
        _ => None,
    })?;
    Some((route, kind))
}

fn configure_contexts(function: &mut ItemFn, contexts: &[Context], mediator: &Ident) {
    function.sig.generics.params.push(parse_quote!(#mediator));
    for (_, route, kind) in contexts {
        let predicate = kind.bound(route, mediator);
        function
            .sig
            .generics
            .make_where_clause()
            .predicates
            .push(parse_quote!(#predicate));
    }
    for argument in &mut function.sig.inputs {
        let FnArg::Typed(argument) = argument else { continue };
        if let Some((route, kind)) = medi_ctx_command(&argument.ty) {
            let marker = kind.marker(&route);
            let context_type: Type = parse_quote!(::medi_rs::MediCtx<'_, #marker, #mediator>);
            *argument.ty = context_type;
        }
    }
}

fn ordered_context_arguments(
    original_types: &[Type],
    message_index: usize,
    contexts: &[Context],
    resource_types: &[&Type],
    parameters: &InjectedParameters<'_>,
) -> Vec<proc_macro2::TokenStream> {
    let mut resource_position = 0;
    original_types
        .iter()
        .enumerate()
        .map(|(index, _)| {
            if index == message_index {
                quote! { message }
            } else if let Some((_, route, kind)) = contexts.iter().find(|(ctx_index, _, _)| *ctx_index == index) {
                let marker = kind.marker(route);
                quote! { ::medi_rs::MediCtx::<#marker, _>::new(mediator) }
            } else {
                let resource = resource_types[resource_position];
                let resource_index = &parameters.indexes[resource_position];
                resource_position += 1;
                match resource {
                    Type::Reference(reference) => {
                        let resource = &reference.elem;
                        quote! { ::medi_rs::tlist::get_ref::<#resource, #resource_index, R>(resources) }
                    }
                    _ => quote! { ::medi_rs::tlist::get::<#resource, #resource_index, R>(resources) },
                }
            }
        })
        .collect()
}

pub(crate) fn typed_arguments(function: &ItemFn) -> Vec<&Type> {
    function
        .sig
        .inputs
        .iter()
        .filter_map(|argument| match argument {
            FnArg::Typed(argument) => Some(argument.ty.as_ref()),
            FnArg::Receiver(_) => None,
        })
        .collect()
}

pub fn medi_handler_inner(
    attribute: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let decorators = match syn::parse::<MediHandlerArgs>(attribute) {
        Ok(args) => args.decorators,
        Err(error) => return error.into_compile_error().into(),
    };
    let mut function = parse_macro_input!(input as ItemFn);
    if function.sig.asyncness.is_none() {
        return syn::Error::new_spanned(
            function.sig.fn_token,
            "`#[medi_handler]` can only be applied to an `async fn`",
        )
        .into_compile_error()
        .into();
    }
    if let Some(receiver) = function.sig.inputs.iter().find_map(|argument| match argument {
        FnArg::Receiver(receiver) => Some(receiver),
        FnArg::Typed(_) => None,
    }) {
        return syn::Error::new_spanned(
            receiver,
            "`#[medi_handler]` functions cannot have a `self` receiver; use a resource parameter instead",
        )
        .into_compile_error()
        .into();
    }
    let original_types: Vec<Type> = typed_arguments(&function).into_iter().cloned().collect();
    if original_types.is_empty() {
        return syn::Error::new_spanned(&function.sig, "a mediator handler requires a message parameter")
            .into_compile_error()
            .into();
    }
    let message_index = original_types.len() - 1;
    if medi_ctx_command(&original_types[message_index]).is_some() {
        return syn::Error::new_spanned(
            &original_types[message_index],
            "the final handler parameter must be the message, not a MediCtx",
        )
        .into_compile_error()
        .into();
    }
    let contexts: Vec<_> = original_types
        .iter()
        .enumerate()
        .filter_map(|(index, ty)| medi_ctx_command(ty).map(|(route, kind)| (index, route, kind)))
        .collect();
    let mediator_type =
        (!contexts.is_empty()).then(|| crate::common::fresh_type_parameter(&function.sig.generics, "__MediRsMediator"));
    if let Some(mediator) = &mediator_type {
        configure_contexts(&mut function, &contexts, mediator);
    }

    let name = &function.sig.ident;
    let helper = format_ident!("__medi_handler_{name}");
    let resource_types: Vec<_> = original_types
        .iter()
        .enumerate()
        .filter_map(|(index, ty)| {
            (index != message_index && !contexts.iter().any(|(ctx_index, _, _)| *ctx_index == index)).then_some(ty)
        })
        .collect();
    let parameters = match InjectedParameters::parse(resource_types.clone(), contexts.is_empty()) {
        Ok(parameters) => parameters,
        Err(error) => return error.into_compile_error().into(),
    };
    let message = &original_types[message_index];
    let mediator = mediator_type
        .unwrap_or_else(|| crate::common::fresh_type_parameter(&function.sig.generics, "__MediRsMediator"));
    let route_arg = if contexts.is_empty() {
        quote! {}
    } else {
        quote! { mediator: &#mediator, }
    };
    let handler_call = if contexts.is_empty() {
        parameters.handler_call(&function.sig.ident, &quote! { message })
    } else {
        let ordered_args =
            ordered_context_arguments(&original_types, message_index, &contexts, &resource_types, &parameters);
        quote! { #name(#(#ordered_args),*).await }
    };
    let helper_body = decorate_handler_call(&decorators, quote! { message }, &handler_call);
    let mediator_parameter = if contexts.is_empty() {
        parameters.mediator_parameter()
    } else {
        route_arg
    };
    let helper_generics = if contexts.is_empty() {
        parameters.helper_generics(core::iter::empty())
    } else {
        let indexes = &parameters.indexes;
        quote! { <#mediator, R, #(#indexes,)*> }
    };
    let resource_bounds = parameters.resource_bounds();
    let decorator_resource_bound = (!decorators.is_empty()).then(|| quote! { R: Sync, });
    let output = &function.sig.output;
    let context_bounds = contexts.iter().map(|(_, route, kind)| {
        let bound = kind.bound(route, &mediator);
        quote! { #bound, }
    });

    quote! {
        #function

        #[doc(hidden)]
        pub async fn #helper #helper_generics(
            #mediator_parameter
            resources: &R,
            message: #message,
        ) #output
        where
            #(#resource_bounds)*
            #decorator_resource_bound
            #(#context_bounds)*
        {
            #helper_body
        }
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::{MediCtxKind, medi_ctx_command};
    use quote::ToTokens;
    use syn::parse_quote;

    #[test]
    fn parses_each_context_route_kind() {
        let command = medi_ctx_command(&parse_quote!(MediCtx<MediCommand<Refresh>>)).unwrap();
        assert_eq!(command.0.to_token_stream().to_string(), "Refresh");
        assert!(matches!(command.1, MediCtxKind::Command));
        let event = medi_ctx_command(&parse_quote!(MediCtx<MediEvent<Changed>>)).unwrap();
        assert_eq!(event.0.to_token_stream().to_string(), "Changed");
        assert!(matches!(event.1, MediCtxKind::Event));
        let stream = medi_ctx_command(&parse_quote!(MediCtx<MediStream<Search>>)).unwrap();
        assert_eq!(stream.0.to_token_stream().to_string(), "Search");
        assert!(matches!(stream.1, MediCtxKind::Stream));
    }

    #[test]
    fn rejects_non_context_and_unknown_context_markers() {
        assert!(medi_ctx_command(&parse_quote!(Repository)).is_none());
        assert!(medi_ctx_command(&parse_quote!(MediCtx<Unknown<Command>>)).is_none());
        assert!(medi_ctx_command(&parse_quote!(MediCtx<MediEvent>)).is_none());
    }
}

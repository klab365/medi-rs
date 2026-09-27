//! `#[medi_handler]` parsing and expansion.

use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{FnArg, Ident, ItemFn, Path, Result as SynResult, Token, Type, bracketed, parse_macro_input};

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
    context: Option<&'a Type>,
    resources: Vec<&'a Type>,
    indexes: Vec<Ident>,
}

impl<'a> InjectedParameters<'a> {
    /// Split the parameters preceding the handler's message (and, for stream
    /// handlers, its sender) into an optional mediator context and resources.
    pub(crate) fn parse(arguments: Vec<&'a Type>) -> SynResult<Self> {
        let (context, resources): (Option<&Type>, Vec<&Type>) = match arguments.first() {
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
    let function = parse_macro_input!(input as ItemFn);
    let name = &function.sig.ident;
    let helper = format_ident!("__medi_handler_{name}");
    let mut arguments = typed_arguments(&function);

    if arguments.is_empty() {
        return syn::Error::new_spanned(&function.sig, "a mediator handler requires a message parameter")
            .into_compile_error()
            .into();
    }

    let message = arguments.pop().expect("checked above");
    let parameters = match InjectedParameters::parse(arguments) {
        Ok(parameters) => parameters,
        Err(error) => return error.into_compile_error().into(),
    };

    let handler_call = parameters.handler_call(name, &quote! { message });
    let helper_body = decorate_handler_call(&decorators, quote! { message }, &handler_call);
    let mediator_parameter = parameters.mediator_parameter();
    let helper_generics = parameters.helper_generics(core::iter::empty());
    let resource_bounds = parameters.resource_bounds();
    // Decorator continuations must be `Send`; when they capture `resources`,
    // the referenced resource tuple must therefore be `Sync`.
    let decorator_resource_bound = (!decorators.is_empty()).then(|| quote! { R: Sync, });
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
            #decorator_resource_bound
        {
            #helper_body
        }
    }
    .into()
}

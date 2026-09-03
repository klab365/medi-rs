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

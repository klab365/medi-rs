//! `#[medi_task]` parsing and expansion.

use quote::{format_ident, quote};
use syn::{FnArg, Ident, ItemFn, Type, parse_macro_input};

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
    let (context, resources): (Option<&Type>, Vec<&Type>) = match arguments.first() {
        Some(Type::Reference(reference)) => (Some(reference.elem.as_ref()), arguments[1..].to_vec()),
        _ => (None, arguments),
    };
    let indexes: Vec<Ident> = (0..resources.len()).map(|index| format_ident!("I{index}")).collect();
    let call_arguments = resources.iter().zip(&indexes).map(|(resource, index)| {
        quote! { ::medi_rs::tlist::get::<#resource, #index, R>(resources) }
    });
    let resource_bounds = resources.iter().zip(&indexes).map(|(resource, index)| {
        quote! { R: ::medi_rs::tlist::Get<#resource, #index>, }
    });
    let mediator_parameter = match context {
        Some(context) => quote! { mediator: &#context, },
        None => quote! { _mediator: &M, },
    };
    let helper_generics = if context.is_some() {
        quote! { <R, #(#indexes,)*> }
    } else {
        quote! { <M, R, #(#indexes,)*> }
    };
    let task_call = if context.is_some() {
        quote! { #name(mediator, #(#call_arguments,)*).await }
    } else {
        quote! { #name(#(#call_arguments,)*).await }
    };

    quote! {
        #function

        #[doc(hidden)]
        pub(crate) async fn #helper #helper_generics(
            #mediator_parameter
            resources: &R,
        )
        where
            #(#resource_bounds)*
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

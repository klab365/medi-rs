//! `#[medi_startup]` and `#[medi_shutdown]` expansion.

use crate::handler::{InjectedParameters, typed_arguments};
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{ItemFn, parse_macro_input};

fn runtime_is_enabled() -> bool {
    cfg!(any(feature = "tokio", feature = "wasm", feature = "embassy"))
}

pub(crate) fn hook_invoker_path(hook: &syn::Path, phase: &str) -> syn::Path {
    let mut invoker = hook.clone();
    let last = invoker
        .segments
        .last_mut()
        .expect("a syn::Path always has at least one segment");
    last.ident = format_ident!("__medi_{phase}_{}", last.ident);
    last.arguments = syn::PathArguments::None;
    invoker
}

pub fn medi_hook_inner(
    phase: &str,
    attribute: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    if !runtime_is_enabled() {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "lifecycle hooks require a medi-rs runtime feature",
        )
        .into_compile_error()
        .into();
    }
    if !attribute.is_empty() {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "lifecycle hooks do not accept arguments",
        )
        .into_compile_error()
        .into();
    }

    let function = parse_macro_input!(input as ItemFn);
    if function.sig.asyncness.is_some() {
        return syn::Error::new(
            function.sig.asyncness.span(),
            "lifecycle hooks must be synchronous functions",
        )
        .into_compile_error()
        .into();
    }
    let name = &function.sig.ident;
    let helper = format_ident!("__medi_{phase}_{name}");
    let parameters = match InjectedParameters::parse(typed_arguments(&function)) {
        Ok(parameters) => parameters,
        Err(error) => return error.into_compile_error().into(),
    };
    let call_arguments = parameters
        .resources
        .iter()
        .zip(&parameters.indexes)
        .map(|(resource, index)| match resource {
            syn::Type::Reference(reference) => {
                let resource = &reference.elem;
                quote! { ::medi_rs::tlist::get_ref::<#resource, #index, R>(resources), }
            }
            _ => quote! { ::medi_rs::tlist::get::<#resource, #index, R>(resources), },
        });
    let resource_bounds = parameters.resource_bounds();
    let mediator_parameter = parameters.mediator_parameter();
    let helper_generics = parameters.helper_generics(core::iter::empty());
    let hook_call = if parameters.context.is_some() {
        quote! { #name(mediator, #(#call_arguments)*) }
    } else {
        quote! { #name(#(#call_arguments)*) }
    };

    quote! {
        #function

        #[doc(hidden)]
        pub(crate) fn #helper #helper_generics(#mediator_parameter resources: &R)
        where
            #(#resource_bounds)*
        {
            let (): () = #hook_call;
        }
    }
    .into()
}

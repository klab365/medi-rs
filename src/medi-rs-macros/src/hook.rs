//! `#[medi_startup]` and `#[medi_shutdown]` expansion.

use crate::handler::{InjectedParameters, typed_arguments};
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{ItemFn, Type, parse_macro_input};

fn runtime_is_enabled() -> bool {
    cfg!(any(feature = "tokio", feature = "wasm", feature = "embassy"))
}

fn is_startup_spawner(ty: &Type) -> bool {
    matches!(ty,
        Type::Reference(reference)
            if reference.mutability.is_none()
                && matches!(reference.elem.as_ref(), Type::Path(path)
                    if path.qself.is_none()
                        && path.path.segments.last().is_some_and(|segment| segment.ident == "StartupSpawner")))
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
    let mut arguments = typed_arguments(&function);
    let uses_startup_spawner = arguments.first().is_some_and(|argument| is_startup_spawner(argument));
    if uses_startup_spawner {
        if phase != "startup" {
            return syn::Error::new_spanned(
                arguments[0],
                "`StartupSpawner` is only available to `#[medi_startup]` hooks",
            )
            .into_compile_error()
            .into();
        }
        if !cfg!(feature = "embassy") {
            return syn::Error::new_spanned(arguments[0], "`StartupSpawner` requires the `embassy` runtime feature")
                .into_compile_error()
                .into();
        }
        arguments.remove(0);
    }
    let parameters = match InjectedParameters::parse(arguments) {
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
    let startup_spawner_parameter =
        (phase == "startup").then(|| quote! { startup_spawner: &::medi_rs::StartupSpawner, });
    let hook_call = match (uses_startup_spawner, parameters.context.is_some()) {
        (true, true) => quote! { #name(startup_spawner, mediator, #(#call_arguments)*) },
        (true, false) => quote! { #name(startup_spawner, #(#call_arguments)*) },
        (false, true) => quote! { #name(mediator, #(#call_arguments)*) },
        (false, false) => quote! { #name(#(#call_arguments)*) },
    };
    let helper_body = if phase == "startup" {
        quote! { ::medi_rs::StartupHookResult::into_start_result(#hook_call, stringify!(#name)) }
    } else {
        quote! { let (): () = #hook_call; }
    };
    let output = if phase == "startup" {
        quote! { -> core::result::Result<(), ::medi_rs::StartError> }
    } else {
        quote! {}
    };

    quote! {
        #function

        #[doc(hidden)]
        pub(crate) fn #helper #helper_generics(#startup_spawner_parameter #mediator_parameter resources: &R) #output
        where
            #(#resource_bounds)*
        {
            #helper_body
        }
    }
    .into()
}

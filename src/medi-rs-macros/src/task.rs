//! `#[medi_task]` parsing and expansion.

use quote::{format_ident, quote};
use syn::{FnArg, Ident, ItemFn, Type, parse_macro_input};

struct TaskParameters<'a> {
    context: Option<&'a Type>,
    signal: bool,
    resources: Vec<&'a Type>,
}

fn runtime_is_enabled() -> bool {
    cfg!(any(feature = "tokio", feature = "wasm", feature = "embassy"))
}

fn is_shutdown_signal(ty: &Type) -> bool {
    matches!(ty, Type::Reference(reference) if matches!(reference.elem.as_ref(), Type::Path(path) if path.path.segments.last().is_some_and(|segment| segment.ident == "ShutdownSignal")))
}

fn typed_arguments(function: &ItemFn) -> Vec<&Type> {
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

fn split_parameters<'a>(arguments: Vec<&'a Type>) -> TaskParameters<'a> {
    let (context, remaining) = match arguments.first() {
        Some(Type::Reference(reference)) if !is_shutdown_signal(arguments[0]) => {
            (Some(reference.elem.as_ref()), arguments[1..].to_vec())
        }
        _ => (None, arguments),
    };
    let (signal, resources) = match remaining.first() {
        Some(signal) if is_shutdown_signal(signal) => (true, remaining[1..].to_vec()),
        None | Some(_) => (false, remaining),
    };
    TaskParameters {
        context,
        signal,
        resources,
    }
}

fn generate_task_call(
    name: &Ident,
    has_context: bool,
    has_signal: bool,
    call_arguments: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    match (has_context, has_signal) {
        (true, true) => quote! { #name(mediator, signal, #call_arguments).await },
        (true, false) => quote! { #name(mediator, #call_arguments).await },
        (false, true) => quote! { #name(signal, #call_arguments).await },
        (false, false) => quote! { #name(#call_arguments).await },
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

    let function = parse_macro_input!(input as ItemFn);
    let name = &function.sig.ident;
    let helper = format_ident!("__medi_task_{name}");
    let parameters = split_parameters(typed_arguments(&function));
    let indexes: Vec<Ident> = (0..parameters.resources.len())
        .map(|index| format_ident!("I{index}"))
        .collect();
    let call_arguments = parameters.resources.iter().zip(&indexes).map(|(resource, index)| {
        quote! { ::medi_rs::tlist::get::<#resource, #index, R>(resources), }
    });
    let resource_bounds = parameters.resources.iter().zip(&indexes).map(|(resource, index)| {
        quote! { R: ::medi_rs::tlist::Get<#resource, #index>, }
    });
    let mediator_parameter = match parameters.context {
        Some(context) => quote! { mediator: &#context, },
        None => quote! { _mediator: &M, },
    };
    let helper_generics = if parameters.context.is_some() {
        quote! { <R, #(#indexes,)*> }
    } else {
        quote! { <M, R, #(#indexes,)*> }
    };
    let task_call = generate_task_call(
        name,
        parameters.context.is_some(),
        parameters.signal,
        &quote! { #(#call_arguments)* },
    );

    quote! {
        #function

        #[doc(hidden)]
        pub(crate) async fn #helper #helper_generics(
            #mediator_parameter
            resources: &R,
            signal: &'static ::medi_rs::ShutdownSignal,
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

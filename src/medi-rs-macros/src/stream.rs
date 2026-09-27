//! `MediStreamRequest` derive and `#[medi_stream_handler]` expansion.

use crate::handler::{InjectedParameters, typed_arguments};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{DeriveInput, Expr, ItemFn, Type, parse_macro_input, parse_quote};

const UNSUPPORTED_ATTRIBUTE: &str =
    "unsupported medi_stream attribute; expected `item_type = Type`, `error_type = Type`, or `capacity = expr`";

pub fn derive_medi_stream_request_inner(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let mut item_type: Option<Type> = None;
    let mut error_type: Option<Type> = None;
    let mut capacity: Option<Expr> = None;
    for attr in input.attrs.iter().filter(|attr| attr.path().is_ident("medi_stream")) {
        let parsed = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("item_type") {
                item_type = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("error_type") {
                error_type = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("capacity") {
                capacity = Some(meta.value()?.parse()?);
            } else {
                return Err(meta.error(UNSUPPORTED_ATTRIBUTE));
            }
            Ok(())
        });
        if let Err(error) = parsed {
            return error.to_compile_error().into();
        }
    }

    let Some(item_type) = item_type else {
        return syn::Error::new_spanned(
            &input.ident,
            "a stream request requires `#[medi_stream(item_type = Type)]`",
        )
        .to_compile_error()
        .into();
    };
    let error_type = error_type.unwrap_or_else(|| parse_quote!(::core::convert::Infallible));
    let capacity = capacity.unwrap_or_else(|| parse_quote!(1));

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    quote! {
        impl #impl_generics ::medi_rs::StreamRequest for #name #ty_generics #where_clause {
            type Item = #item_type;
            type Error = #error_type;
            const CAPACITY: usize = {
                let capacity: usize = #capacity;
                assert!(capacity > 0, "medi_stream capacity must be greater than zero");
                capacity
            };
        }
    }
    .into()
}

pub fn medi_stream_handler_inner(attribute: TokenStream, input: TokenStream) -> TokenStream {
    if !attribute.is_empty() {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "`#[medi_stream_handler]` does not accept arguments",
        )
        .into_compile_error()
        .into();
    }

    let function = parse_macro_input!(input as ItemFn);
    let name = &function.sig.ident;
    let helper = format_ident!("__medi_stream_handler_{name}");
    let mut arguments = typed_arguments(&function);
    if arguments.len() < 2 {
        return syn::Error::new_spanned(
            &function.sig,
            "a stream handler requires a `StreamSender` parameter followed by the request parameter",
        )
        .into_compile_error()
        .into();
    }

    let request = arguments.pop().expect("checked above");
    let sender = arguments.pop().expect("checked above");
    let parameters = match InjectedParameters::parse(arguments) {
        Ok(parameters) => parameters,
        Err(error) => return error.into_compile_error().into(),
    };

    let handler_call = parameters.handler_call(name, &quote! { sender, request });
    let mediator_parameter = parameters.mediator_parameter();
    // Handler lifetimes may appear in the sender type, e.g. `StreamSender<'a, R>`.
    let helper_generics = parameters.helper_generics(function.sig.generics.lifetimes());
    let resource_bounds = parameters.resource_bounds();
    let output = &function.sig.output;

    quote! {
        #function

        #[doc(hidden)]
        pub(crate) async fn #helper #helper_generics(
            #mediator_parameter
            resources: &R,
            sender: #sender,
            request: #request,
        ) #output
        where
            #(#resource_bounds)*
        {
            #handler_call
        }
    }
    .into()
}

pub(crate) fn stream_handler_invoker_path(handler: &syn::Path) -> syn::Path {
    let mut invoker = handler.clone();
    let last = invoker
        .segments
        .last_mut()
        .expect("a syn::Path always has at least one segment");
    last.ident = format_ident!("__medi_stream_handler_{}", last.ident);
    last.arguments = syn::PathArguments::None;
    invoker
}

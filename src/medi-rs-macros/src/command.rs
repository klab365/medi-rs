//! `MediCommand` derive expansion.

use proc_macro::TokenStream;
use quote::quote;
use syn::{Attribute, DeriveInput, Type, parse_macro_input, parse_quote};

pub fn derive_medi_command_inner(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let return_type = match extract_return_type(&input.attrs) {
        Ok(Some(return_type)) => return_type,
        Ok(None) => parse_quote!(()),
        Err(error) => return error.to_compile_error().into(),
    };
    let error_type = match extract_error_type(&input.attrs) {
        Ok(Some(error_type)) => error_type,
        Ok(None) => parse_quote!(::core::convert::Infallible),
        Err(error) => return error.to_compile_error().into(),
    };

    let expanded = quote! {
        impl #impl_generics ::medi_rs::Command for #name #ty_generics #where_clause {
            type Response = #return_type;
        }

        impl #impl_generics ::medi_rs::StaticCommand for #name #ty_generics #where_clause {
            type Error = #error_type;
        }
    };

    TokenStream::from(expanded)
}

fn extract_return_type(attrs: &[Attribute]) -> syn::Result<Option<Type>> {
    let mut return_type = None;

    for attr in attrs {
        if !attr.path().is_ident("medi_command") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("return_type") {
                let value = meta.value()?;
                return_type = Some(value.parse::<Type>()?);
                Ok(())
            } else if meta.path.is_ident("error_type") {
                // Parsed separately by `extract_error_type`.
                let value = meta.value()?;
                let _: Type = value.parse()?;
                Ok(())
            } else {
                Err(meta
                    .error("unsupported medi_command attribute; expected `return_type = Type` or `error_type = Type`"))
            }
        })?;
    }

    Ok(return_type)
}

fn extract_error_type(attrs: &[Attribute]) -> syn::Result<Option<Type>> {
    let mut error_type = None;

    for attr in attrs {
        if !attr.path().is_ident("medi_command") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("error_type") {
                let value = meta.value()?;
                error_type = Some(value.parse::<Type>()?);
                Ok(())
            } else if meta.path.is_ident("return_type") {
                let value = meta.value()?;
                let _: Type = value.parse()?;
                Ok(())
            } else {
                Err(meta
                    .error("unsupported medi_command attribute; expected `return_type = Type` or `error_type = Type`"))
            }
        })?;
    }

    Ok(error_type)
}

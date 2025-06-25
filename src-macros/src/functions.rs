use proc_macro::TokenStream;
use syn::{Attribute, DeriveInput, Meta, Type, parse_macro_input, parse_quote};

pub fn derive_medi_command_inner(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Look for #[medi_command(return_type = SomeType)] attribute
    let return_type = extract_return_type(&input.attrs).unwrap_or_else(|| parse_quote!(()));

    let expanded = quote::quote! {
        impl #impl_generics IntoCommand<#return_type> for #name #ty_generics #where_clause {}
    };

    TokenStream::from(expanded)
}

fn extract_return_type(attrs: &[Attribute]) -> Option<Type> {
    for attr in attrs {
        if !attr.path().is_ident("medi_command") {
            continue;
        }

        let Meta::List(meta_list) = &attr.meta else {
            continue;
        };

        // Parse the attribute arguments manually from tokens
        let tokens = &meta_list.tokens;
        let tokens_str = tokens.to_string();

        // Look for "return_type = TypeName" pattern (without quotes)
        let Some(eq_pos) = tokens_str.find('=') else {
            continue;
        };

        let key = tokens_str[..eq_pos].trim();
        if key != "return_type" {
            continue;
        }

        let type_str = tokens_str[eq_pos + 1..].trim();
        // Parse the type string directly
        if let Ok(ty) = syn::parse_str::<Type>(type_str) {
            return Some(ty);
        }
    }
    None
}

pub fn derive_medi_event_inner(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let expanded = quote::quote! {
        impl #impl_generics IntoEvent for #name #ty_generics #where_clause {}
    };

    TokenStream::from(expanded)
}

pub fn derive_medi_ressource_inner(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let expanded = quote::quote! {
        impl #impl_generics FromResources for #name #ty_generics #where_clause {}
    };

    TokenStream::from(expanded)
}

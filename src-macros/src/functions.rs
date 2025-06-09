use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};


pub fn derive_medi_command_inner(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input
}

pub fn derive_medi_event_inner(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input
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

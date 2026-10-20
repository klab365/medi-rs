//! Shared helpers used by procedural-macro expansions.

use quote::format_ident;
use syn::{GenericParam, Generics, Ident};

/// Choose a type parameter name that does not shadow a parameter in `generics`.
pub(crate) fn fresh_type_parameter(generics: &Generics, base: &str) -> Ident {
    let mut suffix = 0;
    loop {
        let candidate = if suffix == 0 {
            format_ident!("{base}")
        } else {
            format_ident!("{base}_{suffix}")
        };
        let used = generics.params.iter().any(|parameter| match parameter {
            GenericParam::Type(parameter) => parameter.ident == candidate,
            GenericParam::Const(parameter) => parameter.ident == candidate,
            GenericParam::Lifetime(parameter) => parameter.lifetime.ident == candidate,
        });
        if !used {
            return candidate;
        }
        suffix += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::fresh_type_parameter;

    #[test]
    fn keeps_base_when_it_is_free() {
        let generics: syn::Generics = syn::parse_quote!(<T>);
        assert_eq!(fresh_type_parameter(&generics, "Mediator"), "Mediator");
    }

    #[test]
    fn avoids_type_const_and_lifetime_parameter_names() {
        let generics: syn::Generics = syn::parse_quote!(<'a, Mediator, const Mediator_1: usize>);
        assert_eq!(fresh_type_parameter(&generics, "Mediator"), "Mediator_2");
    }
}

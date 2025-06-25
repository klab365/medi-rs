mod functions;

use functions::{derive_medi_command_inner, derive_medi_event_inner, derive_medi_ressource_inner};

#[proc_macro_derive(MediCommand, attributes(medi_command))]
pub fn derive_medi_command(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_medi_command_inner(input)
}

#[proc_macro_derive(MediEvent)]
pub fn derive_medi_event(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_medi_event_inner(input)
}

#[proc_macro_derive(MediRessource)]
pub fn derive_medi_ressource(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_medi_ressource_inner(input)
}

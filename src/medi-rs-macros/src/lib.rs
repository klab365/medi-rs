//! Derive macros for `medi-rs`.
//!
//! These macros generate command metadata, handler invokers, and module
//! manifests for the main `medi-rs` crate.

mod functions;

use functions::{
    derive_medi_command_inner, medi_handler_inner, medi_module_inner, medi_task_inner,
    mediator_composition_marker_inner,
};

/// Derive static command metadata for a command or query type.
///
/// Use `#[medi_command(return_type = Type)]` to specify the response type. If
/// omitted, the command response type is `()`.
#[proc_macro_derive(MediCommand, attributes(medi_command))]
pub fn derive_medi_command(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_medi_command_inner(input)
}

/// Declare a reusable mediator registration manifest owned by a Rust module.
///
/// A manifest is consumed by the `medi_rs::mediator!` macro.
#[proc_macro]
pub fn medi_module(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    medi_module_inner(input)
}

/// Generate a typed static-dispatch invoker for an async handler function.
///
/// Use `#[medi_handler(decorators = [logging, validate])]` to wrap an
/// invocation with middleware functions that receive the command and a
/// continuation.
#[proc_macro_attribute]
pub fn medi_handler(attribute: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    medi_handler_inner(attribute, input)
}

/// Generate a runtime task invoker with typed mediator resource injection.
///
/// The selected medi-rs runtime starts registered tasks. The first parameter
/// must be `&AppMediator`; remaining value parameters are resources.
#[proc_macro_attribute]
pub fn medi_task(attribute: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    medi_task_inner(attribute, input)
}

/// Internal endpoint for the `mediator!` manifest collector.
#[doc(hidden)]
#[proc_macro]
pub fn mediator_composition_marker(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    mediator_composition_marker_inner(input)
}

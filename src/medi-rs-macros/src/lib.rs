//! Derive macros for `medi-rs`.
//!
//! These macros generate command metadata and actor-module manifests for the
//! main `medi-rs` crate.

mod actor;
mod command;
mod composition;
mod generate;
#[allow(dead_code)]
mod handler;
#[allow(dead_code)]
mod hook;
mod manifest;
#[allow(dead_code)]
mod stream;
#[allow(dead_code)]
mod task;

use command::derive_medi_command_inner;
use composition::finalize_composition_inner;
use manifest::medi_module_inner;

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
/// A manifest is consumed by the `medi_rs::mediator!` macro. Use
/// `pub manifest name;` when the manifest is composed from another crate. In
/// that form, use `crate::` paths in the declarations; they are resolved to
/// the defining crate when the exported manifest expands. Handler functions
/// may remain private; only their generated, documentation-hidden invokers
/// are public.
#[proc_macro]
pub fn medi_module(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    medi_module_inner(input)
}

/// Internal endpoint that validates a collected manifest graph and generates a mediator.
#[doc(hidden)]
#[proc_macro]
pub fn __medi_rs_finalize_composition(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    finalize_composition_inner(input)
}

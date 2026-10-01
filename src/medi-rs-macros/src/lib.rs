//! Derive macros for `medi-rs`.
//!
//! These macros generate command metadata, handler invokers, and module
//! manifests for the main `medi-rs` crate.

mod command;
mod composition;
mod generate;
mod handler;
mod hook;
mod manifest;
mod stream;
mod task;

use command::derive_medi_command_inner;
use composition::finalize_composition_inner;
use handler::medi_handler_inner;
use hook::medi_hook_inner;
use manifest::medi_module_inner;
use stream::{derive_medi_stream_request_inner, medi_stream_handler_inner};
use task::medi_task_inner;

/// Derive static command metadata for a command or query type.
///
/// Use `#[medi_command(return_type = Type)]` to specify the response type. If
/// omitted, the command response type is `()`.
#[proc_macro_derive(MediCommand, attributes(medi_command))]
pub fn derive_medi_command(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_medi_command_inner(input)
}

/// Derive static stream metadata for a streaming request type.
///
/// `#[medi_stream(item_type = Type)]` selects the yielded item type and is
/// required. `error_type = Type` defaults to `core::convert::Infallible`, and
/// `capacity = expr` (the number of buffered items) defaults to `1` and must be
/// a const expression greater than zero.
#[proc_macro_derive(MediStreamRequest, attributes(medi_stream))]
pub fn derive_medi_stream_request(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_medi_stream_request_inner(input)
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

/// Generate a typed static-dispatch invoker for an async handler function.
///
/// Use `#[medi_handler(decorators = [logging, validate])]` to wrap an
/// invocation with middleware functions that receive the command and a
/// continuation.
#[proc_macro_attribute]
pub fn medi_handler(attribute: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    medi_handler_inner(attribute, input)
}

/// Register a synchronous startup hook with typed mediator and resource injection.
///
/// Hooks are listed in a module's `startup` section and run in composition
/// order before event workers and runtime tasks are spawned. A hook may return
/// `Result<(), E>`; its error makes `start` return
/// `StartError::StartupHookFailed`. With the `embassy` feature, its first
/// parameter may be `&medi_rs::StartupSpawner`, a temporary executor-local
/// spawner that exists only during `start(spawner)`.
#[proc_macro_attribute]
pub fn medi_startup(attribute: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    medi_hook_inner("startup", attribute, input)
}

/// Register a synchronous shutdown hook with typed mediator and resource injection.
///
/// Hooks are listed in a module's `shutdown` section and run in composition
/// order after workers and runtime tasks have returned.
#[proc_macro_attribute]
pub fn medi_shutdown(attribute: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    medi_hook_inner("shutdown", attribute, input)
}

/// Generate a typed static-dispatch invoker for an async stream handler.
///
/// The last parameter is the stream request and the one before it is its
/// `medi_rs::StreamSender`. Earlier parameters follow `#[medi_handler]`: an
/// optional first `&AppMediator`, then value or `&` resources. The handler
/// returns `Result<(), Error>` with the request's error type.
#[proc_macro_attribute]
pub fn medi_stream_handler(
    attribute: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    medi_stream_handler_inner(attribute, input)
}

/// Generate a runtime task invoker with typed mediator resource injection.
///
/// The selected medi-rs runtime starts registered tasks. An optional first
/// `&AppMediator` parameter provides mediator access; remaining value parameters
/// are resources.
#[proc_macro_attribute]
pub fn medi_task(attribute: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    medi_task_inner(attribute, input)
}

/// Internal endpoint that validates a collected manifest graph and generates a mediator.
#[doc(hidden)]
#[proc_macro]
pub fn __medi_rs_finalize_composition(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    finalize_composition_inner(input)
}

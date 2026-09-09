#![cfg_attr(not(feature = "std"), no_std)]

//! Static async mediator with generated command dispatch, event workers, and
//! typed resource injection.
//!
//! Applications define commands with [`MediCommand`], annotate async handlers
//! with [`medi_handler`], group routes in [`medi_module`], and compose the
//! selected modules with [`mediator!`]. The generated mediator dispatches
//! commands directly to their handler and resolves handler resources from a
//! typed tuple; it does not use dynamic handler or resource registries.
//!
//! # Commands
//!
//! ```no_run
//! use medi_rs::{MediCommand, Result, medi_handler, medi_module, mediator};
//!
//! #[derive(MediCommand)]
//! #[medi_command(return_type = String, error_type = medi_rs::Error)]
//! struct Greet;
//!
//! #[medi_handler]
//! async fn greet(_: Greet) -> Result<String> {
//!     Ok("hello".into())
//! }
//!
//! medi_module! {
//!     manifest greeting;
//!     commands { Greet => greet; }
//! }
//!
//! mediator! {
//!     struct AppMediator {
//!         event_queue_capacity: 16;
//!         event_workers: 1;
//!         modules: [greeting];
//!     }
//! }
//!
//! # async fn run() -> Result<()> {
//! assert_eq!(AppMediator::new().send(Greet).await?, "hello");
//! # Ok(())
//! # }
//! ```
//!
//! Commands default to a `()` response and [`core::convert::Infallible`] error.
//! Set `return_type` and `error_type` in `#[medi_command(...)]` for other
//! response and application-error types.
//!
//! # Resources and events
//!
//! Resources are ordinary `Clone` values listed in a module's `resources`
//! section. They are passed to the generated `new` constructor and injected as
//! handler parameters before the message. Events are ordinary `Clone + Send +
//! 'static` values listed in an `events` section. To process events, retain the
//! mediator in `'static` storage and call its generated `start` method before
//! calling `publish`.
//!
//! Enable one runtime feature for event processing: `tokio`, `wasm`, or
//! `embassy`. The features are mutually exclusive. Command-only mediators do
//! not need a runtime feature.

#[cfg(test)]
extern crate alloc;

/// Embassy executor re-export used by the generated Embassy integration.
#[cfg(feature = "embassy")]
#[doc(hidden)]
pub use embassy_executor;

/// Compose module-owned mediator manifests into one application mediator.
///
/// Each manifest is declared by [`medi_rs_macros::medi_module!`]. The explicit
/// list is the application's routing boundary: only listed modules participate
/// in the generated mediator. Optionally, `decorators: [logging]` applies each
/// listed decorator function to every command and event handler route.
/// `event_failure_reporter: reporter;` observes failed event handlers without
/// changing delivery to the remaining handlers.
#[macro_export]
macro_rules! mediator {
    (
        $vis:vis struct $name:ident {
            event_queue_capacity: $capacity:expr;
            event_workers: $workers:expr;
            modules: [$first:ident $(, $rest:ident)* $(,)?];
            $(decorators: [$($decorators:path),* $(,)?];)?
            $(event_failure_reporter: $reporter:ty;)?
        }
    ) => {
        $first!($crate::__medi_rs_collect_modules, {
            $vis struct $name;
            event_queue_capacity: $capacity;
            event_workers: $workers;
            modules: [];
            decorators: [$($($decorators),*)?];
            event_failure_reporter: [$($reporter)?];
            count: [];
            remaining: [$($rest),*];
        });
    };
}

/// Internal continuation used by [`mediator!`].
#[doc(hidden)]
#[macro_export]
macro_rules! __medi_rs_collect_modules {
    (
        $vis:vis struct $name:ident;
        event_queue_capacity: $capacity:expr;
        event_workers: $workers:expr;
        modules: [$($modules:tt)*];
        decorators: [$($decorators:path),*];
        event_failure_reporter: [$($reporter:ty)?];
        count: [$($count:tt)*];
        remaining: [];
    ) => {
        $crate::__medi_rs_finalize_composition! {
            $vis struct $name;
            event_queue_capacity: $capacity;
            event_workers: $workers;
            modules: [$($modules)*];
            decorators: [$($decorators),*];
            event_failure_reporter: [$($reporter)?];
            count: [$($count)*];
        }
    };
    (
        $vis:vis struct $name:ident;
        event_queue_capacity: $capacity:expr;
        event_workers: $workers:expr;
        modules: [$($modules:tt)*];
        decorators: [$($decorators:path),*];
        event_failure_reporter: [$($reporter:ty)?];
        count: [$($count:tt)*];
        remaining: [$next:ident $(, $rest:ident)*];
    ) => {
        $next!($crate::__medi_rs_collect_modules, {
            $vis struct $name;
            event_queue_capacity: $capacity;
            event_workers: $workers;
            modules: [$($modules)*];
            decorators: [$($decorators),*];
            event_failure_reporter: [$($reporter)?];
            count: [$($count)*];
            remaining: [$($rest),*];
        });
    };
}

#[cfg(any(
    all(feature = "tokio", feature = "wasm"),
    all(feature = "tokio", feature = "embassy"),
    all(feature = "wasm", feature = "embassy")
))]
compile_error!("features `tokio`, `wasm`, and `embassy` are mutually exclusive; enable at most one runtime adapter");

pub mod adapters;
mod bus;
mod error;
mod event;
mod handler;
mod resource;
/// Internal typed-tuple primitives used by generated mediator code.
#[doc(hidden)]
pub mod tlist;

// flatten the module structure
pub use adapters::lifecycle::Lifecycle;
pub use adapters::queue::EventQueue;
pub use adapters::shutdown::ShutdownSignal;
pub use error::*;
pub use handler::*;

pub use medi_rs_macros::{__medi_rs_finalize_composition, MediCommand, medi_handler, medi_module, medi_task};

/// Metadata about a failed asynchronous event-handler invocation.
///
/// Event handlers may use unrelated error types, so this deliberately contains
/// route metadata rather than the handler's concrete error value. Reporters
/// can use it for logging, metrics, and alerting without imposing a common
/// application error type on every event route.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EventHandlerFailure {
    event_name: &'static str,
    handler_name: &'static str,
}

impl EventHandlerFailure {
    /// Create failure metadata for an event route and handler.
    #[doc(hidden)]
    pub const fn new(event_name: &'static str, handler_name: &'static str) -> Self {
        Self {
            event_name,
            handler_name,
        }
    }

    /// The registered event type as written in its module manifest.
    pub const fn event_name(self) -> &'static str {
        self.event_name
    }

    /// The registered handler path as written in its module manifest.
    pub const fn handler_name(self) -> &'static str {
        self.handler_name
    }
}

/// Observes failures from generated asynchronous event-handler dispatch.
///
/// A reporter is configured with `event_failure_reporter:` in [`mediator!`].
/// It is awaited once for each failed handler, but reporting never prevents
/// dispatch to later handlers. The returned future must be `Send` so the same
/// reporter works with Tokio workers; `async fn` implementations satisfy this
/// when their future is `Send`.
pub trait EventFailureReporter: Send + Sync + 'static {
    /// Observe one failed event-handler invocation.
    fn report(&self, failure: EventHandlerFailure) -> impl core::future::Future<Output = ()> + Send;
}

/// Continuation supplied to a function decorator.
///
/// Decorators call [`Self::call`] to forward a command to the next decorator
/// or the handler. The blanket implementation means an ordinary `FnOnce`
/// closure generated by [`medi_handler`] implements this trait automatically.
pub trait DecoratorNext<C>: Send {
    /// Response returned by the remaining decorator pipeline or handler.
    type Response: Send;
    /// Error returned by the remaining decorator pipeline or handler.
    type Error: Send;

    /// Forward `command` through the remaining pipeline.
    fn call(
        self,
        command: C,
    ) -> impl core::future::Future<Output = core::result::Result<Self::Response, Self::Error>> + Send;
}

impl<C, F, Fut, Response, Error> DecoratorNext<C> for F
where
    F: FnOnce(C) -> Fut + Send,
    Fut: core::future::Future<Output = core::result::Result<Response, Error>> + Send,
    Response: Send,
    Error: Send,
{
    type Response = Response;
    type Error = Error;

    fn call(self, command: C) -> impl core::future::Future<Output = core::result::Result<Response, Error>> + Send {
        self(command)
    }
}

/// Command metadata used by generated mediators.
///
/// Most users should derive this with `#[derive(MediCommand)]` from the
/// `medi-rs-macros` crate.
pub trait Command
where
    Self: Send + Sync + 'static,
{
    /// Response type returned by the command handler.
    type Response: Send + Sync + 'static;
}

/// Static-dispatch metadata for a command.
///
/// [`MediCommand`] derives this trait automatically. When its `error_type`
/// attribute is omitted, [`core::convert::Infallible`] is used.
pub trait StaticCommand: Command {
    /// Concrete application error returned by this command's handler.
    type Error: Send;
}

/// Static route generated for a command and a concrete mediator type.
///
/// This is implemented by `mediator!`; applications call the generated
/// mediator's inherent `send` method instead of implementing it directly.
#[doc(hidden)]
pub trait StaticSendCommand<M>: Sized {
    /// Value returned by the command handler.
    type Response;

    /// Concrete error returned by the command handler.
    type Error;

    /// Invoke this command's generated route.
    fn send(
        self,
        mediator: &M,
    ) -> impl core::future::Future<Output = core::result::Result<Self::Response, Self::Error>> + Send;
}

/// Static event route generated for an event and a concrete mediator type.
#[doc(hidden)]
pub trait StaticPublish<M>: Sized {
    /// Enqueue this event for the generated worker.
    fn publish(self, mediator: &M) -> impl core::future::Future<Output = Result<()>> + Send;
}

/// Static non-blocking event route generated for an event and a mediator type.
#[doc(hidden)]
pub trait StaticTryPublish<M>: Sized {
    /// Attempt to enqueue this event without waiting for queue capacity.
    fn try_publish(self, mediator: &M) -> core::result::Result<(), TryPublishError<Self>>;
}

//-- region: Implement static handler traits
crate::impl_static_handler!();
crate::impl_static_handler!(T1: I1);
crate::impl_static_handler!(T1: I1, T2: I2);
crate::impl_static_handler!(T1: I1, T2: I2, T3: I3);
crate::impl_static_handler!(T1: I1, T2: I2, T3: I3, T4: I4);
crate::impl_static_handler!(T1: I1, T2: I2, T3: I3, T4: I4, T5: I5);
crate::impl_static_handler!(T1: I1, T2: I2, T3: I3, T4: I4, T5: I5, T6: I6);
crate::impl_static_handler!(T1: I1, T2: I2, T3: I3, T4: I4, T5: I5, T6: I6, T7: I7);
//-- endregion: Implement the handler traits

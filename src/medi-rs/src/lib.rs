#![cfg_attr(not(feature = "std"), no_std)]

//! Static async mediator built from state-owning actor modules.
//!
//! A [`medi_module`] declaration names a module type, a bounded mailbox, and
//! routes to its `&mut self` methods. [`mediator`] composes those modules;
//! commands use `send` for a reply or `post` for non-blocking delivery, while
//! events use `publish`. See `docs/v4-actors.md` for the complete model.

#[cfg(test)]
extern crate alloc;

/// Embassy executor re-export used by the generated Embassy integration.
#[cfg(feature = "embassy")]
#[doc(hidden)]
pub use embassy_executor;

/// Thread-safe Embassy task spawner.
///
/// Register this as a mediator resource when handlers or runtime tasks need
/// to spawn additional `Send` Embassy tasks. Create it with
/// [`embassy_executor::Spawner::make_send`].
#[cfg(feature = "embassy")]
pub use embassy_executor::SendSpawner;

/// Compose module-owned mediator manifests into one application mediator.
///
/// Each manifest is declared by [`medi_rs_macros::medi_module!`]. The explicit
/// list is the application's routing boundary: only listed modules participate
/// in the generated mediator. Optionally, `decorators: [logging]` applies each
/// listed decorator function to every command and event handler route.
/// `event_failure_reporter: reporter;` observes failed event handlers without
/// changing delivery to the remaining handlers.
/// Module entries may use `#[cfg(...)]`, for example
/// `modules: [core, #[cfg(feature = "extras")] extras];`.
/// Disabled modules are not resolved; an empty module list is also supported.
#[macro_export]
macro_rules! mediator {
    (
        $vis:vis struct $name:ident {
            event_queue_capacity: $capacity:expr;
            event_workers: $workers:expr;
            modules: [$($(#[cfg($condition:meta)])* $module:ident),* $(,)?];
            $(decorators: [$($decorators:path),* $(,)?];)?
            $(event_failure_reporter: $reporter:ty;)?
        }
    ) => {
        $crate::__medi_rs_collect_modules! {
            $vis struct $name;
            event_queue_capacity: $capacity;
            event_workers: $workers;
            modules: [];
            decorators: [$($($decorators),*)?];
            event_failure_reporter: [$($reporter)?];
            count: [];
            remaining: [$($(#[cfg($condition)])* $module,)*];
        }
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
        remaining: [$(#[cfg($condition:meta)])* $next:ident, $($rest:tt)*];
    ) => {
        #[cfg(all($($condition),*))]
        $next!($crate::__medi_rs_collect_modules, {
            $vis struct $name;
            event_queue_capacity: $capacity;
            event_workers: $workers;
            modules: [$($modules)*];
            decorators: [$($decorators),*];
            event_failure_reporter: [$($reporter)?];
            count: [$($count)*];
            remaining: [$($rest)*];
        });
        #[cfg(not(all($($condition),*)))]
        $crate::__medi_rs_collect_modules! {
            $vis struct $name;
            event_queue_capacity: $capacity;
            event_workers: $workers;
            modules: [$($modules)*];
            decorators: [$($decorators),*];
            event_failure_reporter: [$($reporter)?];
            count: [$($count)*];
            remaining: [$($rest)*];
        }
    };
}

#[cfg(any(
    all(feature = "tokio", feature = "wasm"),
    all(feature = "tokio", feature = "embassy"),
    all(feature = "wasm", feature = "embassy")
))]
compile_error!("features `tokio`, `wasm`, and `embassy` are mutually exclusive; enable at most one runtime adapter");

pub mod adapters;
mod error;
/// Internal typed-tuple primitives used by generated mediator code.
#[doc(hidden)]
pub mod tlist;

// flatten the module structure
pub use adapters::lifecycle::Lifecycle;
pub use adapters::mailbox::Mailbox;
pub use adapters::queue::EventQueue;
pub use adapters::shutdown::ShutdownSignal;
#[doc(hidden)]
pub use adapters::startup::StartupSpawner;
pub use error::*;

pub use medi_rs_macros::{__medi_rs_finalize_composition, MediCommand, medi_module};

#[doc(hidden)]
pub struct MissingResource;
#[doc(hidden)]
pub struct ProvidedResource<T>(pub T);

/// The kind of a route in a generated mediator composition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RouteKind {
    /// A request/response command route.
    Command,
    /// A request producing a stream of items.
    Stream,
    /// An asynchronously published event route.
    Event,
}

/// Static metadata for one generated route.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RouteDescription {
    /// The route's command, stream-request, or event type as written in its manifest.
    pub message_name: &'static str,
    /// Whether this is a command, stream, or event route.
    pub kind: RouteKind,
    /// Handler paths in invocation order.
    pub handler_names: &'static [&'static str],
}

/// Static metadata for a generated mediator composition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompositionDescription {
    /// Name of the generated mediator type.
    pub mediator_name: &'static str,
    /// Number of manifests selected by `mediator!`.
    pub module_count: usize,
    /// Declared resource types in constructor order.
    pub resource_names: &'static [&'static str],
    /// Command, stream, and event routes in dispatch order.
    pub routes: &'static [RouteDescription],
}

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

/// Completion summary returned by [`mediator!`]-generated
/// [`publish_and_wait`](macro@mediator) methods.
///
/// Event-handler error types may differ, so this reports counts rather than
/// carrying a concrete application error. Configure an [`EventFailureReporter`]
/// when the application also needs route names for failed handlers.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EventDispatchOutcome {
    succeeded_handlers: usize,
    failed_handlers: usize,
}

impl EventDispatchOutcome {
    /// Create an event-dispatch completion summary.
    #[doc(hidden)]
    pub const fn new(succeeded_handlers: usize, failed_handlers: usize) -> Self {
        Self {
            succeeded_handlers,
            failed_handlers,
        }
    }

    /// Number of handlers that completed successfully.
    pub const fn succeeded_handlers(self) -> usize {
        self.succeeded_handlers
    }

    /// Number of handlers that returned an error.
    pub const fn failed_handlers(self) -> usize {
        self.failed_handlers
    }

    /// Whether every handler completed successfully.
    pub const fn is_success(self) -> bool {
        self.failed_handlers == 0
    }
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
/// closure implements this trait automatically.
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

    /// Concrete application error returned by the command handler.
    ///
    /// [`MediCommand`] uses [`core::convert::Infallible`] when its `error_type`
    /// attribute is omitted.
    type Error: Send;
}

/// Static route generated for a command and a concrete mediator type.
///
/// This is implemented by `mediator!`; applications call the generated
/// mediator's inherent `send` method instead of implementing it directly.
#[doc(hidden)]
pub trait StaticSendCommand<M>: Command + Sized {
    /// Invoke this command's generated route.
    fn send(
        self,
        mediator: &M,
    ) -> impl core::future::Future<Output = core::result::Result<Self::Response, SendError<Self::Error>>> + Send;

    /// Return the actor-module route, if this command targets a module actor.
    fn target_module() -> Option<usize> {
        None
    }
}

/// Static non-blocking command route generated for actor modules.
#[doc(hidden)]
pub trait StaticPostCommand<M>: Command + Sized {
    /// Attempt to enqueue this command without waiting for mailbox capacity.
    fn post(self, mediator: &M) -> core::result::Result<(), PostError<Self>>;
}

/// Handler context for actor modules.
///
/// The context records the currently executing module so awaited self-dispatch
/// can fail immediately instead of deadlocking that module's actor.
pub struct MediContext<'a, M> {
    mediator: &'a M,
    source_module: usize,
}

impl<'a, M> MediContext<'a, M> {
    /// Create a context for generated actor dispatch.
    #[doc(hidden)]
    pub const fn __new(mediator: &'a M, source_module: usize) -> Self {
        Self {
            mediator,
            source_module,
        }
    }

    /// Send a command to another module and await its response.
    pub async fn send<C>(&self, command: C) -> core::result::Result<C::Response, SendError<C::Error>>
    where
        C: StaticSendCommand<M>,
    {
        if C::target_module() == Some(self.source_module) {
            return Err(SendError::Dispatch(DispatchError::SelfDispatch));
        }
        command.send(self.mediator).await
    }

    /// Post a command without waiting for its handler or mailbox capacity.
    pub fn post<C>(&self, command: C) -> core::result::Result<(), PostError<C>>
    where
        C: StaticPostCommand<M>,
    {
        command.post(self.mediator)
    }

    /// Publish an event through the composed mediator's event routes.
    pub async fn publish<E>(&self, event: E) -> Result<()>
    where
        E: StaticPublish<M>,
    {
        event.publish(self.mediator).await
    }
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

/// Static event route that waits for all generated event handlers to finish.
///
/// This is available with the Tokio and Wasm runtime adapters. Embassy's
/// allocation-free event queue does not support per-event completion handles.
#[cfg(any(feature = "tokio", feature = "wasm"))]
#[doc(hidden)]
pub trait StaticPublishAndWait<M>: Sized {
    /// Enqueue this event and await completion of all its handlers.
    fn publish_and_wait(self, mediator: &M) -> impl core::future::Future<Output = Result<EventDispatchOutcome>> + Send;
}

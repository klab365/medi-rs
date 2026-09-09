# Changelog

All notable changes to `medi-rs` are documented here.

## 3.0.0

### Breaking changes

- Merged `StaticCommand` into `Command`. Command implementations now declare
  both `Command::Response` and `Command::Error`; `StaticCommand` is removed.
- Removed the unused `StaticHandler`, `Dependency`, and `impl_static_handler!`
  extension API. Generated `#[medi_handler]` invokers remain the supported
  typed resource-injection mechanism.

### Changed

- `StaticSendCommand` now inherits the response and error types from `Command`
  instead of declaring duplicate associated types.

## 2.2.0

### Added

- Event mediators can now configure `event_failure_reporter: ReporterType;`.
  The generated worker invokes `EventFailureReporter` once for every failed
  event handler while continuing to dispatch the remaining handlers. The
  reporter receives `EventHandlerFailure` event and handler metadata; concrete
  handler errors remain route-local because event routes need not share an
  error type.
- Generated mediators now expose `is_started()` and make `start()` idempotency
  explicit: the first call starts event workers and `#[medi_task]` tasks, while
  later calls return `StartError::AlreadyStarted` without spawning duplicate
  work on Tokio, Wasm, or Embassy.
- Generated event mediators now provide non-blocking `try_publish(event)`.
  It returns `TryPublishError::Full(event)` when the bounded queue has no
  capacity and `TryPublishError::Closed(event)` after shutdown, retaining the
  event for application-defined retry or overload handling. Tokio, Wasm, and
  Embassy adapters provide the same full/closed semantics.
- Generated event mediators now provide `shutdown().await`. Shutdown rejects
  new publishes, drains events accepted before shutdown, and waits for event
  workers and registered `#[medi_task]` tasks to return on Tokio, Wasm, and
  Embassy.
- `#[medi_task]` functions can declare an injected `&ShutdownSignal`, which
  `shutdown()` cancels before waiting for task completion.

## 2.1.0

### Changed

- Mediator constructors now take declared resources as separate arguments, such
  as `Mediator::new(resource_a, resource_b)`, instead of one resource tuple.
- Updated the Embassy integration and the micro:bit example to compatible
  `embassy-executor` 0.10, `embassy-nrf` 0.11, `embassy-sync` 0.8, and
  `embassy-time` 0.5 releases. Embassy consumers must update from the 0.7 spawn
  API and enable the appropriate `platform-*` executor feature.

## 2.0.0

### Breaking changes

- Replaced the allocator-backed dynamic bus, handler registry, and resource
  registry with macro-generated static mediators. `Bus` and `BusBuilder` are
  removed; define routes with `medi_module!` and compose them with `mediator!`.
- Removed the `MediResource` and `MediRessource` derives, `MediEvent`, the
  dynamic command and event traits, and dynamic framework handler errors.
  Resources and events are now ordinary values.
- Commands must derive `MediCommand` and now return the concrete error type
  declared with `#[medi_command(error_type = ...)]`, rather than a framework
  handler error. The response and error defaults are `()` and `Infallible`.
- Resources must be `Clone`, declared in a module manifest, and passed to the
  generated mediator constructor in manifest order. Resource lookup is now
  compile-time typed rather than registry- or `TypeId`-based.
- Events must be `Clone + Send + 'static`, declared in a module manifest, and
  require a selected runtime feature and a started, `'static` mediator before
  they can be published. Event-handler errors are ignored after dispatch.
- Runtime support is opt-in: the default feature set is runtime-free. Enable
  exactly one of `tokio`, `wasm`, or `embassy` for events and runtime tasks.

### Added

- Static, direct command dispatch and typed resource injection generated at
  compile time.
- Reusable module-owned manifests, explicit mediator composition boundaries,
  and compile-time diagnostics for duplicate commands/resources and missing
  handler resources.
- `#[medi_handler]` for generating typed async handler invokers, including
  optional mediator injection and support for private handlers.
- Handler decorators through `DecoratorNext`, configurable per handler or for
  every route in a `mediator!` composition.
- `#[medi_task]` and module task registration with typed resource and optional
  mediator injection.
- Bounded event queues and generated workers for Tokio, WebAssembly, and
  Embassy, plus `no_std`/Embassy support.
- Tokio, WebAssembly, and Embassy micro:bit examples, expanded integration and
  macro-diagnostic tests, and architecture, development, and release docs.

### Changed

- Reorganized the repository as a Cargo workspace containing the `medi-rs`
  library crate and the `medi-rs-macros` proc-macro crate.
- Replaced Just-based development and release tasks with `mise`; CI now checks
  no-runtime, Tokio, WebAssembly, and Embassy configurations, and release CI
  publishes both workspace crates from lowercase `v*.*.*` tags.

## 1.2.0

### Added

- Macros for the former dynamic command and resource APIs.

## 1.1.0

### Changed

- Improved dynamic API error handling.

## 1.0.0

- Initial release.

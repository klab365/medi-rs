# Architecture

`medi-rs` generates a concrete mediator from a fixed set of module manifests.
The generated type owns its resource tuple and, when events are registered, its
typed event queue. This replaces dynamic handler/resource registries with
compile-time routes.

## Crates

- `medi-rs` exposes the public traits, `mediator!`, queue abstraction, and
  runtime adapters.
- `medi-rs-macros` provides `MediCommand`, `#[medi_handler]`, and
  `medi_module!`.

## Composition boundary

A feature module declares a manifest:

```rust
medi_module! {
    manifest users;
    resources { repository: UserRepository; }
    commands { CreateUser => create_user; }
    events { UserCreated => [send_welcome_email, write_audit_log]; }
}
```

The application explicitly selects its modules:

```rust
mediator! {
    pub struct AppMediator {
        event_queue_capacity: 16;
        event_workers: 1;
        modules: [users, audit];
    }
}
```

This explicit list is the routing boundary. Commands, streams, resources, and
events not included through a listed manifest are unavailable to that
mediator. Duplicate command, stream, or resource registrations are rejected
while expanding the macro. The generated mediator exposes this fixed graph
through `COMPOSITION` and `composition()`, static metadata for its resource
types and command, stream, and event routes. This supports tests and tooling
without adding a runtime registry.

## Commands and handlers

`#[derive(MediCommand)]` implements `Command` for the request type.
`Command::Response` is selected with `#[medi_command(return_type = Type)]`;
`Command::Error` is selected with `error_type = Type`. The defaults are `()`
and `Infallible`.

`#[medi_handler]` keeps the original async function and emits a typed invoker.
The final argument is the command or event. Earlier value arguments are cloned
from the mediator's resource tuple. An optional first `&Mediator` argument lets
a handler send commands or publish events through its own mediator.

For a command route, `mediator.send(command).await` invokes exactly the one
registered handler and returns that handler's concrete `Result<Response,
Error>`. There is no framework-wide boxed handler error.

## Streams

`#[derive(MediStreamRequest)]` implements `StreamRequest` with the item type,
error type, and channel capacity. `#[medi_stream_handler]` emits an invoker
that injects the optional mediator and resources exactly like a command
handler, followed by a `StreamSender` and the request.

For every stream route, `mediator!` adds one field holding the selected
runtime's `StreamChannel`, created in `new`, and implements a route used by the
generated `stream` method. `stream` returns a composition of `futures`
combinators that, on first poll, leases the route's channel and then polls the
handler future and the channel receiver together. No task is spawned and
nothing is allocated per stream. The handler's final `Ok(())` or `Err(error)`
is queued behind its items as a terminal message, which orders an error after
every item already sent. Dropping the stream drops the handler future and the
lease; the next lease holder clears leftover items.

## Resources

Resources are ordinary `Clone` values, not marker-derived types. Each resource
is named and listed once in `resources { ... }`, then supplied through the
generated typed builder. The macro represents the values as a typed nested
tuple and the handler invoker resolves dependencies through type-level tuple
positions.

As a result, duplicate resources and missing dependencies fail to compile.
Resource extraction does not use `TypeId`, a map, or allocation.

## Events

For every event type in the selected manifests, `mediator!` generates a variant
in a private event-job enum and a `publish` route. Calling `publish` enqueues
the event and returns after it is accepted by the selected adapter queue.

Call `start` on a `'static` mediator to launch `EVENT_WORKERS` generated
workers. A worker receives a job and invokes every registered handler for that
event. Event values must be `Clone + Send + 'static`, because each handler
receives its own clone. A failing handler never stops dispatch to the remaining
handlers and does not make the earlier `publish` call fail: `publish` only
confirms queue acceptance. By default failures are discarded for compatibility.

Applications can add `event_failure_reporter: ReporterType;` to `mediator!`.
The generated worker awaits its `EventFailureReporter::report` method exactly
once after every failed handler. Reporting also cannot stop later handlers.
`EventHandlerFailure` contains the manifest-written event and handler names;
it deliberately does not contain the concrete error because independent routes
may use unrelated error types. This supports logging, metrics, and alerting but
is not a retry, dead-letter, or error-unification mechanism.

## Lifecycle hooks

When a runtime feature is enabled, manifests can list synchronous `startup`
and `shutdown` hooks. The `#[medi_startup]` and `#[medi_shutdown]` attributes
generate typed invokers with the same optional mediator and resource injection
model as handlers. Generated `start` runs startup hooks once in
manifest-composition order before spawning event workers and runtime tasks. The
initiating `shutdown` call runs shutdown hooks once, in the same order, after
all workers and tasks have returned.

## Runtime adapters

The runtime feature selects the queue and worker-spawn implementation:

- `tokio`: bounded Tokio MPSC queue and `tokio::spawn`.
- `wasm`: futures MPSC queue and `wasm_bindgen_futures::spawn_local`.
- `embassy`: Embassy channel and generated `#[embassy_executor::task]` workers.

The same feature selects the stream channel: `TokioStreamChannel`
(`tokio::sync::mpsc`), `WasmStreamChannel` (`futures::channel::mpsc`), or
`EmbassyStreamChannel` (`embassy_sync::channel::Channel`, capacity as a const
generic). Stream handler senders are independent of the capacity, so handlers
never name a const generic.

`tokio`, `wasm`, and `embassy` are mutually exclusive. Embassy queues have a
fixed adapter capacity; the macro's capacity setting is accepted for a uniform
manifest syntax but does not change that queue's capacity.

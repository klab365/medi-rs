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
    resources { UserRepository; }
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

This explicit list is the routing boundary. Commands, resources, and events
not included through a listed manifest are unavailable to that mediator.
Duplicate command or resource registrations are rejected while expanding the
macro.

## Commands and handlers

`#[derive(MediCommand)]` implements `Command` and `StaticCommand` for the
request type. `Command::Response` is selected with
`#[medi_command(return_type = Type)]`; `StaticCommand::Error` is selected with
`error_type = Type`. The defaults are `()` and `Infallible`.

`#[medi_handler]` keeps the original async function and emits a typed invoker.
The final argument is the command or event. Earlier value arguments are cloned
from the mediator's resource tuple. An optional first `&Mediator` argument lets
a handler send commands or publish events through its own mediator.

For a command route, `mediator.send(command).await` invokes exactly the one
registered handler and returns that handler's concrete `Result<Response,
Error>`. There is no framework-wide boxed handler error.

## Resources

Resources are ordinary `Clone` values, not marker-derived types. Each resource
is listed once in `resources { ... }` and supplied to `Mediator::new` in the
same declaration order across the composed manifests. The macro represents the
values as a typed nested tuple and the handler invoker resolves dependencies
through type-level tuple positions.

As a result, duplicate resources and missing dependencies fail to compile.
Resource extraction does not use `TypeId`, a map, or allocation.

## Events

For every event type in the selected manifests, `mediator!` generates a variant
in a private event-job enum and a `publish` route. Calling `publish` enqueues
the event and returns after it is accepted by the selected adapter queue.

Call `start` on a `'static` mediator to launch `EVENT_WORKERS` generated
workers. A worker receives a job and invokes every registered handler for that
event. Event values must be `Clone + Send + 'static`, because each handler
receives its own clone. Handler failures are currently discarded so that one
failing event handler does not stop dispatch to the remaining handlers.

## Runtime adapters

The runtime feature selects the queue and worker-spawn implementation:

- `tokio`: bounded Tokio MPSC queue and `tokio::spawn`.
- `wasm`: futures MPSC queue and `wasm_bindgen_futures::spawn_local`.
- `embassy`: Embassy channel and generated `#[embassy_executor::task]` workers.

`tokio`, `wasm`, and `embassy` are mutually exclusive. Embassy queues have a
fixed adapter capacity; the macro's capacity setting is accepted for a uniform
manifest syntax but does not change that queue's capacity.

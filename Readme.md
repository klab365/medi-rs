# medi-rs

`medi-rs` is a static async mediator for Rust. Applications declare commands, events, resources, and handlers in feature-local modules; `mediator!` then combines those manifests into one concrete mediator. Command dispatch and resource injection are generated at compile time—there is no runtime handler registry or type-based resource lookup.

## Documentation

- [Architecture](docs/architecture.md)
- [Development](docs/development.md)
- [Release process](docs/release.md)

## Runtime features

Choose one runtime adapter for event processing:

| Feature | Runtime | Notes |
| --- | --- | --- |
| `tokio` | Tokio | Hosted applications and the runnable Tokio examples. |
| `wasm` | `wasm_bindgen_futures` | WebAssembly event workers use `spawn_local`. |
| `embassy` | Embassy | `no_std` embedded applications; queue capacity must be a const expression. |

The runtime features are mutually exclusive. Command-only mediators need no runtime feature. Events and runtime tasks require exactly one adapter feature.

```toml
[dependencies]
medi-rs = { version = "1", features = ["tokio"] }
```

## Event configuration

Event mediators use a bounded queue. `event_queue_capacity` and `event_workers` must both be greater than zero; publishing waits while the queue is full. Tokio and WebAssembly accept any `usize` expression for the capacity. Embassy uses the capacity as a const generic, so its value must be a const expression such as a literal or named `const`.

## Quick start

A command derives `MediCommand`; its handler is marked with `#[medi_handler]`. A `medi_module!` manifest declares the route, and `mediator!` creates the application mediator.

```rust
use medi_rs::{MediCommand, Result, medi_handler, medi_module, mediator};

#[derive(MediCommand)]
#[medi_command(return_type = String, error_type = medi_rs::Error)]
struct Greet {
    name: String,
}

#[medi_handler]
async fn greet(command: Greet) -> Result<String> {
    Ok(format!("Hello, {}!", command.name))
}

medi_module! {
    manifest greeting;
    commands { Greet => greet; }
}

mediator! {
    pub struct AppMediator {
        event_queue_capacity: 16;
        event_workers: 1;
        modules: [greeting];
    }
}

# async fn run() -> Result<()> {
let greeting = AppMediator::new()
    .send(Greet { name: "Rust".into() })
    .await?;
assert_eq!(greeting, "Hello, Rust!");
# Ok(())
# }
```

`MediCommand` defaults to a `()` response and `core::convert::Infallible` error. Specify `return_type` and `error_type` when the handler returns other types.

## Static-dispatch pattern

The registration graph is fixed where `mediator!` is expanded:

1. Define a message type and derive `MediCommand` for commands.
2. Mark each async handler with `#[medi_handler]`; its final parameter is the command or event it receives.
3. In each feature-local module, use `medi_module!` to list its resources and routes.
4. At the application boundary, select those manifests with `mediator!`.

`mediator!` generates one concrete mediator type. For every command route it implements a route for that command type and mediator type; `send` therefore calls the selected handler directly. Resources live in a typed nested tuple and are cloned into handler parameters by their compile-time tuple position. There is no runtime `TypeId` lookup, boxed handler registry, or handler selection at runtime. A command or resource registered twice is rejected while expanding the composition, and requesting an undeclared handler resource fails type checking.

## Macro reference

### `#[derive(MediCommand)]` and `#[medi_command(...)]`

Derive `MediCommand` on a command or query type. It implements `Command`, which supplies the response type, and `StaticCommand`, which supplies the handler error type:

```rust
#[derive(medi_rs::MediCommand)]
#[medi_command(return_type = User, error_type = CreateUserError)]
struct CreateUser { /* fields */ }
```

Both options are optional: `return_type` defaults to `()` and `error_type` defaults to `core::convert::Infallible`. `return_type` and `error_type` accept Rust type syntax, so application-specific result and error types work without conversion to a framework error.

### `#[medi_handler]`

Apply this attribute to an async function. It retains the function and creates a typed, crate-visible internal invoker used by generated routes, so the handler function itself can remain private to its feature module. The last parameter is always the message. Value parameters before it are resources, which must be listed in the composed manifest and implement `Clone`. Optionally, the first parameter can be `&AppMediator` so a handler can send another command or publish an event.

```rust
#[medi_handler]
async fn create_user(
    mediator: &AppMediator,
    repository: UserRepository,
    command: CreateUser,
) -> Result<User, CreateUserError> {
    // `repository` is cloned from AppMediator's declared resources.
    mediator.send(RecordAudit).await?;
    repository.create(command).await
}
```

The handler's return type must match the command metadata for a command route. For event routes it should return `Result<()>`; event-handler errors are ignored after the handler completes. To wrap an invocation with cross-cutting behavior, define middleware functions that receive the command and a `next` continuation, then list them in declaration order:

```rust
use medi_rs::DecoratorNext;

async fn logging(
    command: CreateUser,
    next: impl DecoratorNext<CreateUser, Response = User, Error = CreateUserError>,
) -> Result<User, CreateUserError> {
    println!("creating a user");
    next.call(command).await
}

#[medi_handler(decorators = [logging, validation])]
async fn create_user(command: CreateUser) -> Result<User, CreateUserError> {
    // `logging` wraps `validation`, which wraps this handler.
    # todo!()
}
```

`next` is the remaining decorator pipeline and handler. Calling `next.call(command).await` forwards the command; a decorator can reject or modify the command before forwarding it, and can run behavior after it returns. The generated continuation is inferred automatically; only the `DecoratorNext<Command>` parameter type must be declared.

To apply a decorator to every command and event handler in one mediator, add it to the mediator composition:

```rust
mediator! {
    struct AppMediator {
        event_queue_capacity: 16;
        event_workers: 1;
        modules: [users];
        decorators: [logging];
    }
}
```

### `medi_module!`

Declare a reusable, feature-local manifest. It contains zero or more `resources`, `commands`, and `events` sections in any order. With a runtime feature, manifests may additionally contain a `tasks` section. Commands have one handler; events have one or more handlers. The macro creates the named manifest for inclusion by `mediator!`.

```rust
medi_module! {
    manifest users;
    resources { UserRepository; Clock; }
    commands { CreateUser => create_user; }
    events { UserCreated => [send_welcome_email, write_audit_log]; }
}
```

Use semicolons between resource and command entries, and commas between event handlers. When a handler is private in a feature module, use its crate-qualified path (for example, `crate::users::create_user`) in the manifest; the generated invoker remains crate-visible while the handler stays private. The manifest contains declarations only: it does not construct a mediator or register anything dynamically.

### `mediator!`

Compose one or more manifests into the concrete application mediator. Its explicit `modules` list is the routing boundary and its order determines the order of resource arguments accepted by `new`.

```rust
mediator! {
    pub struct AppMediator {
        event_queue_capacity: 16;
        event_workers: 1;
        modules: [users, audit];
    }
}
```

The generated type has `new`, `send`, and, when an event route exists, `publish` and `start`. It uses the runtime selected by the enabled `tokio`, `wasm`, or `embassy` feature. The event configuration rules are described in [Event configuration](#event-configuration). `start` requires `'static` mediator storage (`start(spawner)` for Embassy).

`mediator_composition_marker!` is also exported for internal macro expansion. It is not an application-facing API; use `mediator!` instead.

## Resources

Resources are ordinary `Clone` values. List each resource in a module manifest, pass the values to the generated mediator constructor in declaration order, and request them as handler parameters before the command or event.

```rust
# use medi_rs::{MediCommand, Result, medi_handler, medi_module, mediator};
#[derive(Clone)]
struct UserRepository;

#[derive(MediCommand)]
#[medi_command(error_type = medi_rs::Error)]
struct CreateUser;

#[medi_handler]
async fn create_user(_: UserRepository, _: CreateUser) -> Result<()> {
    Ok(())
}

medi_module! {
    manifest users;
    resources { UserRepository; }
    commands { CreateUser => create_user; }
}

mediator! {
    struct AppMediator {
        event_queue_capacity: 16;
        event_workers: 1;
        modules: [users];
    }
}

# async fn run() -> Result<()> {
AppMediator::new((UserRepository,)).send(CreateUser).await?;
# Ok(())
# }
```

A missing or duplicate resource is a compile-time error. Resource derive macros are not required.

### Runtime tasks

With a runtime feature, `#[medi_task]` creates a task with the same typed resource injection as a handler. Its first parameter is `&AppMediator`; remaining value parameters are declared resources. Register it in a `tasks` section. `mediator.start(spawner)` starts tasks on Embassy; `mediator.start()` does so on Tokio and Wasm.

```rust,ignore
#[medi_task]
async fn watch_button(mediator: &AppMediator, board: BoardApi) {
    loop {
        board.wait_for_button_a().await;
        let _ = mediator.send(ButtonPressed).await;
    }
}

medi_module! {
    manifest buttons;
    resources { BoardApi; }
    tasks { watch_button; }
}
```

## Events

Events are plain `Clone + Send + 'static` values. List each event route in a module manifest, create a `'static` mediator, and call `start` before publishing. Each generated worker dispatches an event to every registered handler. `publish` waits when the configured bounded queue is full. Event handler errors are currently ignored after dispatch.

```rust,no_run
# use medi_rs::{Result, medi_handler, medi_module, mediator};
# #[derive(Clone)] struct UserRegistered;
# #[medi_handler] async fn send_welcome_email(_: UserRegistered) -> Result<()> { Ok(()) }
# medi_module! { manifest users; events { UserRegistered => [send_welcome_email]; } }
# mediator! { struct AppMediator { event_queue_capacity: 16; event_workers: 1; modules: [users]; } }
# async fn run() -> Result<()> {
let mediator = Box::leak(Box::new(AppMediator::new()));
mediator.start();
mediator.publish(UserRegistered).await?;
# Ok(())
# }
```

For Embassy, initialize the mediator in a `StaticCell` and call `mediator.start(spawner)`. See the micro:bit example below.

## Examples

- [Tokio](examples/tokio/)
  - [Request/response command](examples/tokio/src/bin/request_response.rs)
  - [Resource injection](examples/tokio/src/bin/resources.rs)
  - [Event dispatch](examples/tokio/src/bin/events.rs)
  - [Typed handler errors](examples/tokio/src/bin/custom_error.rs)
- [WebAssembly](examples/wasm/)
- [Embassy micro:bit v2](examples/embassy-microbit-v2/)

## Development

The repository uses [mise](https://mise.jdx.dev/) for its Rust toolchain and commands:

```sh
mise install
mise run check-format
mise run lint
mise run test
mise run check-examples
mise run run-examples
mise run check-docs
```

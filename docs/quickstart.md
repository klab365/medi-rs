# Quick start

This guide builds a small Tokio application with one command and one injected
resource. It shows the complete static-dispatch workflow: define a command,
write its handler, declare a module manifest, compose a mediator, and send the
command.

## 1. Add the dependency

Commands do not require a runtime. This example uses Tokio, so enable its
adapter:

```toml
[dependencies]
medi-rs = { version = "2", features = ["tokio"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Enable exactly one of `tokio`, `wasm`, or `embassy` when using events, streams,
or runtime tasks. A command-only mediator can omit all three features.

## 2. Define the command and handler

A command derives `MediCommand`. Its last handler parameter is the command;
preceding value parameters are resources supplied when the mediator is
constructed.

```rust
use medi_rs::{MediCommand, Result, medi_handler, medi_module, mediator};

#[derive(Clone)]
struct GreetingPrefix(String);

#[derive(MediCommand)]
#[medi_command(return_type = String, error_type = medi_rs::Error)]
struct Greet {
    name: String,
}

#[medi_handler]
async fn greet(prefix: GreetingPrefix, command: Greet) -> Result<String> {
    Ok(format!("{}, {}!", prefix.0, command.name))
}
```

`MediCommand` defaults to a `()` response and an `Infallible` error. Set
`return_type` and `error_type` only when the handler returns other types.

## 3. Declare and compose the route

`medi_module!` is a feature-local declaration; it does not register handlers at
runtime. `mediator!` selects manifests and generates one concrete mediator.

```rust
medi_module! {
    manifest greeting;
    resources { prefix: GreetingPrefix; }
    commands { Greet => greet; }
}

mediator! {
    struct AppMediator {
        event_queue_capacity: 16;
        event_workers: 1;
        modules: [greeting];
    }
}
```

The resource order in the selected manifests is the argument order of `new`.
A missing resource, duplicate resource, or duplicate command route is rejected
at compile time.

## 4. Send the command

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let mediator = AppMediator::builder().prefix(GreetingPrefix("Hello".into()).build());

    let greeting = mediator.send(Greet { name: "Rust".into() }).await?;
    assert_eq!(greeting, "Hello, Rust!");
    Ok(())
}
```

`send` calls the generated route directly. There is no runtime handler registry
or type-based resource lookup.

## Testing a handler

For Tokio event handlers, `publish_and_wait` waits until dispatch completes, so
there is no need to synchronize a test with sleeps or polling:

```rust
let outcome = mediator.publish_and_wait(MyEvent).await?;
assert_eq!(outcome.succeeded_handlers(), 1);
assert_eq!(outcome.failed_handlers(), 0);
```

Here `mediator` is a mediator with an event route, built with the Tokio or Wasm
adapter.

Use this when the test needs to assert handler effects. Ordinary `publish` only
confirms that the event was accepted by the queue. `publish_and_wait` is
available on Tokio and Wasm, not Embassy.

## Choosing a runtime and examples

- **Tokio**: hosted applications; start with this quick start and the
  [runnable Tokio examples](../examples/tokio/README.md).
- **Embassy**: `no_std` embedded applications; the
  [micro:bit v2 example](../examples/embassy-microbit-v2/README.md) shows static
  mediator storage, resource injection, and `start(spawner)`.
- **Wasm**: browser applications; see the [Wasm example](../examples/wasm/README.md).

## Next steps

- Read the [architecture overview](architecture.md) for the composition model.
- See the root [README](../README.md) for events, streams, decorators, lifecycle
  hooks, and runtime tasks.
- Run the [Tokio examples](../examples/tokio/README.md) for executable command,
  resource, event, and stream examples.

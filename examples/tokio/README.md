# Tokio examples

Runnable examples using the Tokio adapter. They cover commands and resources,
events, tasks, streams, and custom handler errors. For the complete route
setup and execution model, see the [quick start](../../docs/quickstart.md).

## Run one example

```sh
cargo run --manifest-path examples/tokio/Cargo.toml --bin request_response
cargo run --manifest-path examples/tokio/Cargo.toml --bin resources
cargo run --manifest-path examples/tokio/Cargo.toml --bin events
cargo run --manifest-path examples/tokio/Cargo.toml --bin tasks
cargo run --manifest-path examples/tokio/Cargo.toml --bin custom_error
cargo run --manifest-path examples/tokio/Cargo.toml --bin streams
```

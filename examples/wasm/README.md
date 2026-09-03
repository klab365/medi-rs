# medi-rs WASM example

This example composes a static mediator with the `wasm` adapter. Commands use
the generated `send` route; events are queued and processed by workers spawned
with `wasm_bindgen_futures::spawn_local`. It also registers a `#[medi_task]`
startup task, which is spawned through the same runtime when `mediator.start()`
is called.

## Check

```sh
cargo check --manifest-path examples/wasm/Cargo.toml
```

## Test with Rust/WASM tooling

Install `wasm-pack`, then run the `wasm-bindgen-test` tests from this example
directory:

```sh
cd examples/wasm
wasm-pack test --node
```

Node.js is the easiest option because it does not require ChromeDriver or
GeckoDriver.

## Build and run in a browser

A browser cannot load the generated ES module or `.wasm` file from a `file://`
URL. Build it, then serve this directory over HTTP:

```sh
cd examples/wasm
wasm-pack build --target web
python3 -m http.server 8000
```

Open <http://localhost:8000/> and view the browser console. Do not open
`index.html` directly from Finder or with a `file://` URL.

The example exports:

- `greet(name: string): Promise<string>` — command dispatch.
- `greet_with_generated_mediator(name: string): Promise<string>` — the same
  mediator route exposed through a second binding.
- `publish_user_registered(email: string): Promise<void>` — queued event
  publication.

The mediator uses a bounded queue:

```rust
mediator! {
    pub struct WasmMediator {
        event_queue_capacity: 16;
        event_workers: 1;
        modules: [/* manifests */];
    }
}
```

# Embassy micro:bit v2 example

This example runs a static `mediator!` on a BBC micro:bit v2 (`nRF52833`) with
the `embassy` adapter. It uses `StaticCell` for the generated mediator and no
allocator or `extern crate alloc`. It is a hardware-specific end-to-end example,
not a minimal host-side quick start; for the core Embassy setup, see the
[setup summary](#mediator-setup).

## Mediator setup

The application provides its resources to the generated builder, stores the
completed mediator in static memory, then starts its workers with Embassy's
`Spawner`:

```rust,ignore
static MEDIATOR: StaticCell<AppMediator> = StaticCell::new();

let mediator = MEDIATOR.init(
    AppMediator::builder()
        .board(board)
        .button(button)
        .build(),
);
mediator.start(spawner)?;
```

The `StaticCell` gives the mediator the `'static` lifetime required by Embassy
tasks. `start(spawner)` starts the generated event workers and registered
tasks; it does not create OS threads. Resource setter names come from the
resource declarations in the mediator's selected manifests.

`main.rs` bootstraps the concrete `EmbassyBoard` GPIO implementation and injects
it as the `BoardApi` resource. Its `#[medi_startup]` hook receives the temporary
`StartupSpawner` from `mediator.start(spawner)` and uses it to start the
executor-local startup indicator without storing a spawner resource. Its registered `button_monitor` Embassy task
owns and awaits Button A, while its `display` task toggles an LED matrix pixel
through that trait-based API. `button_monitor` sends `ButtonPressed`, whose
handler also uses `BoardApi` and publishes `ButtonObserved`; the generated
Embassy worker invokes the event handler.

After each press, `button_monitor` also opens the `BlinkPattern` stream. Its
`#[medi_stream_handler]` blinks the LED through the injected `BoardApi` and
sends each completed blink through a statically allocated
`embassy_sync::channel::Channel` owned by the mediator. The task logs every
blink as it arrives; every sixth press requests too many blinks, and the
handler's `BlinkError` is yielded as the stream's final item.

## Prerequisites

```sh
rustup target add thumbv7em-none-eabihf
cargo install probe-rs-tools
```

Connect a micro:bit v2 by USB, then run:

```sh
cd examples/embassy-microbit-v2
cargo run --release
```

Press button A to send the command. The count, observed event count, and
streamed blinks are printed through `defmt-rtt`.

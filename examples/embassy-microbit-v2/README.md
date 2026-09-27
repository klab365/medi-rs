# Embassy micro:bit v2 example

This example runs a static `mediator!` on a BBC micro:bit v2 (`nRF52833`) with
the `embassy` adapter. It uses `StaticCell` for the generated mediator and no
allocator or `extern crate alloc`.

`main.rs` bootstraps the concrete `EmbassyBoard` GPIO implementation and injects
it as the `BoardApi` resource. Its registered `button_monitor` Embassy task
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

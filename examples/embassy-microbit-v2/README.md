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

Press button A to send the command. The count and observed event count are
printed through `defmt-rtt`.

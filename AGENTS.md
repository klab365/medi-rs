# AGENTS.md

Guidance for humans and coding agents working in this repository.

## Project overview

`medi-rs` is a Rust mediator library with two workspace crates:

- `medi-rs` — the main library crate in `src/medi-rs/`
- `medi-rs-macros` — derive macros in `src/medi-rs-macros/`

The project uses Rust edition 2024 and the toolchain/tasks from `mise.toml`.

## Required checks

Before opening a PR or finishing an automated change, run:

```sh
mise run check-format
mise run lint
mise run test
mise run check-examples
mise run run-examples
mise run check-docs
```

If you change public documentation or examples, also verify the examples compile where practical.

## Coding guidelines

- Keep `unsafe` out of the codebase; workspace lints forbid it.
- Preserve the public API unless the change is explicitly planned as breaking.
- Prefer clear typed errors over panics in new public APIs.
- Document new public traits, structs, methods, and derive macros.
- Keep runtime behavior changes separate from mechanical refactors.
- Add or update tests for every behavior change.

## Module notes

- `src/medi-rs/src/bus/` owns `Bus` and `BusBuilder` behavior.
- `src/medi-rs/src/handler/` owns handler abstraction and function-handler implementations.
- `src/medi-rs/src/resource/` owns dependency/resource extraction.
- `src/medi-rs/src/event/` owns event wrapping and dispatch support.
- `src/medi-rs-macros/` owns derive macros for commands, events, and resources.

## Test expectations

- Request/response handler changes need integration tests under `src/medi-rs/tests/`.
- Resource injection changes need tests that cover missing and present resources.
- Event changes need tests for multi-handler dispatch and async behavior.
- Macro changes need tests for generated trait implementations and generic/path types.

## Release notes

Versions are managed from workspace metadata in `Cargo.toml`. The existing publish task is:

```sh
mise run publish -- <version> [cargo publish args]
```

Review `docs/release.md` before changing release automation.

# Development

## Prerequisites

This repository uses [`mise`](https://mise.jdx.dev/) to install Rust and cargo tools.

```sh
mise install
```

The configured Rust version and tasks are in `mise.toml`.

## Common commands

```sh
mise run build          # build every supported feature configuration
mise run build -- tokio # build only one configuration (also: no-runtime, wasm, embassy)
mise run check-format   # cargo fmt --all -- --check
mise run format         # cargo fmt --all
mise run lint           # clippy for no_std, Tokio, and WASM feature sets
mise run test           # tests for no_std, Tokio, and WASM feature sets
mise run check-examples # check Tokio and WASM examples
mise run run-examples   # run every Tokio example
mise run check-docs     # build docs for supported feature sets
mise run coverage       # generate an LCOV coverage report
```

`build`, `lint`, `test`, and `check-docs` accept the same optional feature
configuration: `no-runtime`, `tokio`, `wasm`, or `embassy`. Omitting it checks
all configurations.

Before finishing a change, run:

```sh
mise run check-format
mise run lint
mise run test
mise run check-examples
mise run run-examples
mise run check-docs
```

## Workspace layout

- `src/medi-rs/` — main `medi-rs` library crate
- `src/medi-rs-macros/` — `medi-rs-macros` proc-macro crate
- `src/medi-rs/tests/` — integration tests and executable usage examples
- `docs/` — repository documentation and planning notes

## Development rules

- Rust edition is 2024.
- `unsafe` code is forbidden by workspace lints.
- Keep the public API stable unless a breaking change is intentional.
- Prefer small PRs that separate refactoring, behavior changes, and documentation.
- Add tests for new handler, resource, event, or macro behavior.

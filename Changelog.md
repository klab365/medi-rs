# Changelog

All notable changes to `medi-rs` are documented here.

## Unreleased

### Breaking changes

- Replaced the allocator-backed dynamic bus, handler registry, and resource
  registry with macro-generated static mediators.
- Replaced `Bus` and `BusBuilder` with `mediator!`, `medi_module!`, and
  `#[medi_handler]`.
- Removed `MediResource`, the `MediRessource` compatibility spelling, dynamic
  command/event traits, and dynamic framework handler errors.
- Commands now return their handler's concrete error type. Set it with
  `#[medi_command(error_type = ...)]`.
- Resources are ordinary `Clone` values declared in a module manifest; resource
  derives are no longer needed.
- Events are plain `Clone + Send + 'static` values; no event derive is needed.

### Added

- Module-owned manifests and explicit `mediator!` composition.
- Compile-time duplicate command/resource and missing-resource diagnostics.
- Tokio, WASM, and Embassy generated event-worker support.

## 1.3.0

### Added

- Development environment setup.
- API documentation improvements.
- Macro improvements.

## 1.2.0

### Added

- Macros for the former dynamic command and resource APIs.

## 1.1.0

### Changed

- Improved dynamic API error handling.

## 1.0.0

- Initial release.

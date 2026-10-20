# Cross-crate module composition

This example has four independently compiled crates:

- `contracts`: public command and event types shared by features.
- `cache-module`: handles `RefreshCache` and publishes `CacheRefreshed`.
- `startup-module`: sends `RefreshCache` from a startup task using
  `MediCtx<MediCommand<RefreshCache>>`.
- `app`: selects and composes both module manifests into
  `ApplicationMediator`.

Neither feature module names or depends on `ApplicationMediator`. Route binding
is generated statically by `mediator!`; the Embassy-compatible context API does
not require a runtime registry or allocation for dispatch.

Run it with:

```sh
cd examples/module-example
cargo run -p module-example-app
```

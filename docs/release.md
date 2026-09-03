# Release process

Versions are stored in workspace package metadata in `Cargo.toml` and shared by both crates.

## Publish command

The existing release helper is:

```sh
mise run publish -- <version> [cargo publish args]
```

It updates the workspace version and the `medi-rs-macros` dependency version, then runs `cargo publish --workspace --allow-dirty`.

Example dry run:

```sh
mise run publish -- 1.0.1 --dry-run
```

## Current CD workflow

`.github/workflows/cd.yml` publishes when a tag matching `v*.*.*` is pushed.

Use lowercase `v` tags, for example `v1.0.1`.

## Pre-release checklist

1. Update changelog.
2. Run local checks:

   ```sh
   mise run check-format
   mise run lint
   mise run test
   mise run check-examples
   mise run run-examples
   mise run check-docs
   ```

3. Run a publish dry run.
4. Push the release tag.
5. Verify both crates on crates.io.

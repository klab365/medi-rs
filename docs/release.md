# Release process

Versions are stored in workspace package metadata in `Cargo.toml` and shared by both crates.

## Publish command

Set the release version in `Cargo.toml` before creating its tag. The workspace version and the `medi-rs-macros` dependency version must agree.
The release helper publishes those declared versions without modifying files:

```sh
mise run publish -- [cargo publish args]
```

Example dry run:

```sh
mise run publish -- --dry-run
```

Validate the requested version and confirm that no workspace package version is
already published:

```sh
mise run verify-release -- 1.0.1
```

## Current CD workflow

`.github/workflows/cd.yml` publishes when a tag matching `v*.*.*` is pushed. It requires the tag version to match every workspace package and fails if any workspace package version already exists on crates.io.

Use lowercase `v` tags, for example `v1.0.1`.

## Pre-release checklist

1. Update the version in `Cargo.toml` and the changelog.
2. Run local checks:

   ```sh
   mise run check-format
   mise run lint
   mise run test
   mise run check-examples
   mise run run-examples
   mise run check-docs
   ```

3. Run `mise run verify-release -- <version>` and a publish dry run.
4. Push the release tag.
5. Verify both crates on crates.io.

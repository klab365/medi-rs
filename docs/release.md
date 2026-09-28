# Release process

Versions are stored in workspace package metadata in `Cargo.toml` and shared by both crates.

## Release preparation

[Release-plz](https://release-plz.dev/) runs after every push to `main` and
creates or updates a `release-plz-*` pull request. It determines the next
workspace version, updates `Cargo.toml` and `Cargo.lock`, and adds the
conventional-commit release notes to `Changelog.md`. When that pull request is
merged, Release-plz publishes `medi-rs-macros` and then `medi-rs`, creates
crate-specific tags (for example `medi-rs-v2.5.0`), and creates GitHub
releases.

Repository Actions settings must allow workflows to create pull requests (`Read
and write permissions` and `Allow GitHub Actions to create and approve pull
requests`). The `CRATESIO_TOKEN` repository secret must contain a token that
can publish both workspace crates.

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

## Pre-release checklist

1. Merge the Release-plz pull request that updates the version and changelog.
2. Run local checks:

   ```sh
   mise run check-format
   mise run lint
   mise run test
   mise run check-examples
   mise run run-examples
   mise run check-docs
   ```

3. Optionally run `mise run verify-release -- <version>` and a publish dry run.
4. Verify both crates and the generated GitHub releases after Release-plz
   publishes them.

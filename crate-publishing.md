# Publishing workspace dependencies
## Rationale re: versioning
We publish the majority of our workspace dependencies (essentially everything in the repo aside from binaries, smart contracts, and some internal tooling) to [crates.io](https://crates.io).

In order to make this easy to maintain, the versions of these workspace dependencies and the `nym-sdk` crate are kept in sync.

This version is defined in the `[workspace.package]` section of the root monorepo `Cargo.toml` file. Each of the workspace dependencies have their paths and versions (this has to be individually defined at the moment per-dependency, **this version needs to stay the same as the `workspace.package` version**) defined in the `[workspace.dependencies]` section of the root monorepo `Cargo.toml` file.

## When Developing
If you add a workspace dependency to the SDK when developing, make sure to add this to the workspace dependencies in the root monorepo `Cargo.toml`.

## Check local publication
```
# List crates to publish
cargo workspaces list

# Dry run locally - check for compilation or other problems
cargo workspaces publish --no-git-commit --dry-run
```

## CI
There are several workflows that should be run in the following order:
- `publish-crates-io-dry-run`: run this first! This is a remote dry-run on a runner. This greps for any errors that would be a problem when we're not dry-running. It doesn't catch all errors, as `dry-run` has a known issue where, assuming that 2 new crates are being uploaded, and crate B relies on crate A, if crate A isn't on crates.io (which it won't be, since you're dry-running publication), then since `cargo workspace publish` only checks for available versions on crates.io, it will error. We don't want the CI to fail in that case.
- `ci-crates-version-bump`: this bumps the versions of the workspace + dependencies to the passed version, and then commits the change.
- `ci-crates-publish`: this publishes the crates. So long as you're not uploading more than 5 new crates, pass `60` as the `--interval`. This is to get around [crates.io rate limiting](https://github.com/rust-lang/crates.io/blob/ad7e58e1afd65b9137e58a7bca3e1fb7f5546682/src/rate_limiter.rs#L24).

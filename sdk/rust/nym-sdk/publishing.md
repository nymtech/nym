# Publishing the SDK
## Setup / rationale wrt versioning
The Rust SDK has a relatively long (60+) workspace dependencies that have to be published alongside it. In order to make this easy to maintain, the versions of these workspace dependencies and the `nym-sdk` crate (as well as the version of basically all the crates in the monorepo workspace, excluding binaries, contracts, and tools / monitoring clients) are kept in sync.

This version is defined in the `[workspace.package]` section of the root monorepo `Cargo.toml` file. Each of the workspace dependencies have their paths and versions (this has to be individually defined at the moment per-dependency, **this version needs to stay the same as the `workspace.package` version**) defined in the `[workspace.dependencies]` section of the root monorepo `Cargo.toml` file.

We have attempted to automate this as much as possible, but due to limitations in tools like `cargo-smart-release` and publishing a subset of a workspace, this is the initial version of publication logic.

## When Developing
If you add a workspace dependency to the SDK when developing, make sure to add this to the workspace dependencies in the root monorepo `Cargo.toml`.

## Publishing
TODO

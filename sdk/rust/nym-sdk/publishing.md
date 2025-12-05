# Publishing the SDK
## Rationale
The Rust SDK has a relatively long (60+) workspace dependencies that have to be published alongside it. In order to make this easy to maintain, the versions of these workspace dependencies and the `nym-sdk` crate are kept in sync. This is defined in the `[workspace.package]` section of the root monorepo `Cargo.toml` file. Each of the workspace dependencies have their paths and versions (this has to be individually defined at the moment per-dependency, **this version needs to stay the same as the `workspace.package` version**) defined in the `[workspace.dependencies]` section of the root monorepo `Cargo.toml` file.

We have attempted to automate this as much as possible, but due to limitations in tools like `cargo-smart-release` and publishing a subset of a workspace, this is the initial version of publication logic.

## When Developing
**If you add a workspace dependency to the SDK when developing, make sure to add this to the workspace dependencies in the root monorepo `Cargo.toml` file AND the `publish-sdk.sh` script.**

## Publishing
To publish the `nym-sdk` crate:
- bump the `[workspace.package]` version
- bump the versions of the workspace dependencies in the `[workspace.dependencies]` section to the same version
- run `./publish-sdk.sh`:
```
# Dry run with new version, e.g.
./publish-sdk.sh 1.20.0

# If there are no errors, run with execute flag - this will attempt to publish the nym-sdk crate and its dependencies to crates.io
./publish-sdk.sh --execute 1.20.0
```

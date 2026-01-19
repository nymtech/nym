# Publishing workspace dependencies
## Rationale re: versioning
We publish the majority of our workspace dependencies (essentially everything in the repo aside from binaries, smart contracts, and some internal tooling) to [crates.io](https://crates.io).

In order to make this easy to maintain, the versions of these workspace dependencies and the `nym-sdk` crate are kept in sync.

This version is defined in the `[workspace.package]` section of the root monorepo `Cargo.toml` file. Each of the workspace dependencies have their paths and versions (this has to be individually defined at the moment per-dependency, **this version needs to stay the same as the `workspace.package` version**) defined in the `[workspace.dependencies]` section of the root monorepo `Cargo.toml` file.

## When Developing
If you add a workspace dependency to the SDK when developing, make sure to add this to the workspace dependencies in the root monorepo `Cargo.toml`.

## Publishing
```
# List crates to publish
cargo workspaces list

# Dry run - check for compilation or other problems
cargo workspaces publish --no-git-commit --dry-run

# Publish
TODO
```

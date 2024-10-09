# Installation
The `nym-sdk` crate is **not yet available via [crates.io](https://crates.io)**. As such, in order to import the crate you must specify the Nym monorepo in your `Cargo.toml` file. Since the `HEAD` of `master` is always the most recent release, we recommend developers use that for their imports, unless they have a reason to pull in a specific historic version of the code.

```toml
# importing HEAD of master branch
nym-sdk = { git = "https://github.com/nymtech/nym", branch = "master" }
# importing HEAD of the third release of 2023, codename 'kinder'
nym-sdk = { git = "https://github.com/nymtech/nym", branch = "release/2023.3-kinder" }
```

Work will occur in the future to break the monorepo down into importable features, in order to reduce the number of dependencies imported by developers.

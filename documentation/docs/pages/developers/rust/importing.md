# Installation
The `nym-sdk` crate is **not yet available via [crates.io](https://crates.io)**. As such, in order to import the crate you must specify the Nym monorepo in your `Cargo.toml` file:

```toml
nym-sdk = { git = "https://github.com/nymtech/nym" }
```

By default the above command will import the current `HEAD` of the default branch, which in our case is `develop`. Assuming instead you wish to pull in another branch (e.g. `master` or a particular release) you can specify this like so:

```toml
# importing HEAD of master branch
nym-sdk = { git = "https://github.com/nymtech/nym", branch = "master" }
# importing HEAD of the third release of 2023, codename 'kinder'
nym-sdk = { git = "https://github.com/nymtech/nym", branch = "release/2023.3-kinder" }
```

You can also define a particular git commit to use as your import like so:

```toml
nym-sdk = { git = "https://github.com/nymtech/nym", rev = "85a7ec9f02ca8262d47eebb6c3b19d832341b55d" }
```

Since the `HEAD` of `master` is always the most recent release, we recommend developers use that for their imports, unless they have a reason to pull in a specific historic version of the code.

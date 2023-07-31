# Preparing Your Environment

## Prerequisites
* `Rust` & `cargo`

## Creating your Project Structure

* Make a new cargo project:
```
cargo new nym-cosmos-service
```

* Create the following directory structure and files:
```
.
├── Cargo.toml
├── bin
│   ├── client.rs
│   └── service.rs
└── src
    ├── client.rs
    ├── lib.rs
    └── service.rs

3 directories, 6 files
```

* Add the following dependencies to your `Cargo.toml` file:
```
[dependencies]
anyhow = "1.0.72"
clap = { version = "4.0", features = ["derive"] }
bip39 = { version = "2.0.0", features = ["zeroize"] }
cosmrs = "=0.14.0"
TODO
# tokio = { workspace = true, features = ["rt-multi-thread", "macros"] }
tokio = "1.24.1"
bs58 = "0.5.0"
serde = "1.0.152"
serde_json = "1.0.91"
```

These are non Nym-specific dependencies for the project. `anyhow` is for catch-all error handling, `clap` is for setting up the CLI commands, `cosmrs` for cosmos-specific types and functionality, `tokio` for the async/await environment, and `serde` for (de)serialisation.

* Next add Nym-specific dependencies. Since these libraries are not yet on [crates io](https://crates.io) then you need to import them from the Nym monorepo:
```
nym-sdk = { git = "https://github.com/nymtech/nym" }
nym-sphinx-addressing = { git = "https://github.com/nymtech/nym" }
nym-validator-client = { git = "https://github.com/nymtech/nym" }
nym-bin-common = { git = "https://github.com/nymtech/nym" }
nym-sphinx-anonymous-replies = { git = "https://github.com/nymtech/nym" }
```

The `sphinx` dependencies are for packet- and address-related functionality, the `validator-client` for Nyx blockchain specific configs, `common` for client logging, and the `sdk` for SDK functionality: creating and managing client storage and connections, and sending and receiving messages to and from the mixnet.

* Finally add the following underneath your `[dependencies]`:
```
[[bin]]
name = "client"
path = "bin/client.rs"

[[bin]]
name = "service"
path = "bin/service.rs"
```
This defines multiple binaries to run in a single cargo project, as outlined [here](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#binaries).

# Example Cargo File 
This file imports the basic requirements for running these pieces of example code, and can be used as the basis for your own cargo project.

```toml
[package]
name = "your_app"
version = "x.y.z"
edition = "2021"

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

[dependencies]
# Async runtime
tokio = { version = "1.24.1", features = ["rt-multi-thread", "macros"] }
# Used for (de)serialising incoming and outgoing messages 
serde = "1.0.152"
serde_json = "1.0.91"
# Nym clients, addressing, etc 
nym-sdk = { git = "https://github.com/nymtech/nym", branch = "master" }
nym-sphinx-addressing = { git = "https://github.com/nymtech/nym", branch = "master" }
nym-bin-common = { git = "https://github.com/nymtech/nym", branch = "master" }
nym-sphinx-anonymous-replies = { git = "https://github.com/nymtech/nym", branch = "master" }
# Additional dependencies if you're interacting with Nyx or another Cosmos SDK blockchain
cosmrs = "=0.14.0"
nym-validator-client = { git = "https://github.com/nymtech/nym", branch = "master" }

# If you're building an app with a client and server / serivce this might be a useful structure for your repo 
[[bin]]
name = "client"
path = "bin/client.rs"

[[bin]]
name = "service"
path = "bin/service.rs"
```
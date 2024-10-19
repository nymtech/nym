# Nym Echo Server

This is an initial minimal implementation of an echo server built using the `NymProxyServer` Rust SDK abstraction.

## Usage
```
cargo build --release
../../target/release/echo-server <PORT> <PATH_TO_ENV_FILE> e.g.  ../../target/release/echo-server 9000 ../../envs/canary.env
```

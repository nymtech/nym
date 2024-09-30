# Introduction
The Rust SDK allows developers building applications in Rust to import and interact with Nym clients as they would any other dependency, instead of running the client as a separate process on their machine. This makes both developing and running applications much easier, reducing complexity in the development process (not having to restart another client in a separate console window/tab) and being able to have a single binary for other people to use.

Currently developers can use the Rust SDK to import either websocket client ([`nym-client`](../../clients/websocket-client.md)) or [`socks-client`](../../clients/socks5-client.md) functionality into their Rust code.

### Generate Crate Docs
In order to generate the crate docs run `cargo doc --open` from `nym/sdk/rust/nym-sdk/`

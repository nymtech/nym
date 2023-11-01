# Rust SDK
The Rust SDK allows developers building applications in Rust to import and interact with Nym clients as they would any other dependency, instead of running the client as a separate process on their machine. This makes both developing and running applications much easier, reducing complexity in the development process (not having to restart another client in a separate console window/tab) and being able to have a single binary for other people to use.

Currently developers can use the Rust SDK to import either websocket client ([`nym-client`](../../clients/websocket-client.md)) or [`socks-client`](../../clients/socks5-client.md) functionality into their Rust code.

In the future the SDK will be made up of several components, each of which will allow developers to interact with different parts of Nym infrastructure.

| Component | Functionality                                                                         | Released |
|-----------|---------------------------------------------------------------------------------------|----------|
| Mixnet    | Create / load clients & keypairs, subscribe to Mixnet events, send & receive messages | ✔️       |
| Coconut   | Create & verify Coconut credentials                                                   | 🛠️      |
| Validator | Sign & broadcast Nyx blockchain transactions, query the blockchain                    | ❌        |

The `mixnet` component currently exposes the logic of two clients: the [websocket client](../../clients/websocket-client.md), and the [socks](../../clients/socks5-client.md) client.

The `coconut` component is currently being worked on. Right now it exposes logic allowing for the creation of coconut credentials on the Sandbox testnet.

### Development status
The SDK is still somewhat a work in progress: interfaces are fairly stable but still may change in subsequent releases.

### Installation 
The `nym-sdk` crate is **not yet available via [crates.io](https://crates.io)**. As such, in order to import the crate you must specify the Nym monorepo in your `Cargo.toml` file:

TODO add note on branch import for stability - `master` should be last release 
```toml
nym-sdk = { git = "https://github.com/nymtech/nym" }
```

### Generate Crate Docs 
In order to generate the crate docs run `cargo doc --open` from `nym/sdk/rust/nym-sdk/`



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

### Generate Crate Docs 
In order to generate the crate docs run `cargo doc --open` from `nym/sdk/rust/nym-sdk/`



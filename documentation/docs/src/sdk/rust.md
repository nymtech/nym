# Rust SDK
The Rust SDK allows developers building applications in Rust to import and interact with Nym clients as they would any other dependency, instead of running the client as a seperate process on their machine. This makes both developing and running applications much easier, reducing complexity in the development process (not having to restart another client in a seperate console window/tab) and being able to have a single binary for other people to use.

Currently developers can use the Rust SDK to import either websocket client ([`nym-client`](../clients/websocket-client.md)) or [`socks-client`](../clients/socks5-client.md) functionality into their Rust code.

## Development status
The SDK is still somewhat a work in progress: interfaces are fairly stable but still may change in subsequent releases.

The `nym-sdk` crate is **not yet availiable via [crates.io](https://crates.io)**. As such, in order to import the crate you must specify the Nym monorepo in your `Cargo.toml` file:

```toml
nym-sdk = { git = "https://github.com/nymtech/nym" }
```

In order to generate the crate docs run `cargo doc --open` from `nym/sdk/rust/nym-sdk/`

In the future the SDK will be made up of several components, each of which will allow developers to interact with different parts of Nym's infrastructure.

| Component | Functionality                                                                         | Released |
| --------- | ------------------------------------------------------------------------------------- | -------- |
| Mixnet    | Create / load clients & keypairs, subscribe to Mixnet events, send & receive messages | ✔️        |
| Coconut   | Create & verify Coconut credentials                                                   | 🛠️       |
| Validator | Sign & broadcast Nyx blockchain transactions, query the blockchain                    | ❌       |

The `mixnet` component currently exposes the logic of two clients: the websocket client, and the socks client.

The `coconut` component is currently being worked on. Right now it exposes logic allowing for the creation of coconut credentials on the Sandbox testnet.

## Websocket client examples
> All the codeblocks below can be found in the `nym-sdk` [examples directory](https://github.com/nymtech/nym/tree/release/{{platform_release_version}}/sdk/rust/nym-sdk/examples) in the monorepo. Just navigate to `nym/sdk/rust/nym-sdk/examples/` and run the files from there. If you wish to run these outside of the workspace - such as if you want to use one as the basis for your own project - then make sure to import the `sdk`, `tokio`, and `nym_bin_common` crates.

Lets look at a very simple example of how you can import and use the websocket client in a piece of Rust code (`examples/simple.rs`):

```rust,noplayground
{{#include ../../../../sdk/rust/nym-sdk/examples/simple.rs}}
```

Simply importing the `nym_sdk` crate into your project allows you to create a client and send traffic through the mixnet.

### Creating and storing keypairs
The example above involves ephemeral keys - if we want to create and then maintain a client identity over time, our code becomes a little more complex as we need to create, store, and conditionally load these keys (`examples/builder_with_storage`):

```rust,noplayground
{{#include ../../../../sdk/rust/nym-sdk/examples/builder_with_storage.rs}}
```

As seen in the example above, the `mixnet::MixnetClientBuilder::new()` function handles checking for keys in a storage location, loading them if present, or creating them and storing them if not, making client key management very simple.

### Manually handling storage
If you're integrating mixnet functionality into an existing app and want to integrate saving client configs and keys into your existing storage logic, you can manually perform the actions taken automatically above (`examples/manually_handle_keys_and_config.rs`)

```rust,noplayground
{{#include ../../../../sdk/rust/nym-sdk/examples/manually_handle_storage.rs}}
```

### Anonymous replies with SURBs
Both functions used to send messages through the mixnet (`send_str` and `send_bytes`) send a pre-determined number of SURBs along with their messages by default.

The number of SURBs is set [here](https://github.com/nymtech/nym/blob/release/{{platform_release_version}}/sdk/rust/nym-sdk/src/mixnet/client.rs#L34):

```rust,noplayground
{{#include ../../../../sdk/rust/nym-sdk/src/mixnet/client.rs:34}}
```

You can read more about how SURBs function under the hood [here](../architecture/traffic-flow.md#private-replies-using-surbs).

In order to reply to an incoming message using SURBs, you can construct a `recipient` from the `sender_tag` sent along with the message you wish to reply to: 

```rust,noplayground
{{#include ../../../../sdk/rust/nym-sdk/examples/surb-reply.rs}}
```

### Importing and using a custom network topology
If you want to send traffic through a sub-set of nodes (for instance, ones you control, or a small test setup) when developing, debugging, or peforming research, you will need to import these nodes as a custom network topology, instead of grabbing it from the [`Mainnet Nym-API`](https://validator.nymtech.net/api/swagger/index.html) (`examples/custom_topology_provider.rs`).

There are two ways to do this:

#### Import a custom Nym API endpoint
If you are also running a Validator and Nym API for your network, you can specify that endpoint as such and interact with it as clients ususally do (under the hood):

```rust,noplayground
{{#include ../../../../sdk/rust/nym-sdk/examples/custom_topology_provider.rs}}
```

#### Import a specific topology manually
If you aren't running a Validator and Nym API, and just want to import a specific sub-set of mix nodes, you can simply overwrite the grabbed topology manually:

```rust,noplayground
{{#include ../../../../sdk/rust/nym-sdk/examples/manually_overwrite_topology.rs}}
```

## Socks client example
There is also the option to embed the [`socks5-client`](../clients/socks5-client.md) into your app code (`examples/socks5.rs`):

```rust,noplayground
{{#include ../../../../sdk/rust/nym-sdk/examples/socks5.rs}}
```

```admonish info
If you are looking at implementing Nym as a transport layer for a crypto wallet or desktop app, this is probably the best place to start.
```

## Coconut credential generation
The following code shows how you can use the SDK to create and use a [credential](../bandwidth-credentials.md) representing paid bandwidth on the Sandbox testnet.

```rust,noplayground
{{#include ../../../../sdk/rust/nym-sdk/examples/bandwidth.rs}}
```

You can read more about Coconut credentials (also referred to as `zk-Nym`) [here](../coconut.md).

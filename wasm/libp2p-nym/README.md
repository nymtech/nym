# nym-libp2p-wasm

This crate provides a libp2p `Transport` implementation that uses the Nym mixnet for p2p communication in browser environments.

This **implements Nym Client -> Nym Client communication** - using the Mixnet as a 'proxy' (allowing for client-only modifications, and e.g. using existing `/tcp/` multiaddrs for bootnodes and addressing) is not yet available.

## Features

- libp2p `Transport` trait implementation
- Stream multiplexing via `StreamMuxer`
- Message ordering over the unordered mixnet
- Anonymous replies using SURBs
- Browser compatible

## Building

```sh
# Build with wasm-pack (for browser):
cd wasm/libp2p-nym
wasm-pack build --target web
```

## Testing

```sh
# Browser tests (headless)
wasm-pack test --headless --firefox
wasm-pack test --headless --chrome

# If you want to see the logs in a browser window run one of the above commands without the --headless flag and open the window console
```

## Modules

| Module | Description |
|--------|-------------|
| `lib.rs` | Crate root, re-exports public API and JS bindings |
| `client.rs` | Client initialization, connects to Mixnet |
| `transport.rs` | libp2p `Transport` trait implementation |
| `connection.rs` | libp2p `StreamMuxer` implementation |
| `substream.rs` | `AsyncRead`/`AsyncWrite` implementation |
| `mixnet.rs` | Bridges Nym client to async channels |
| `queue.rs` | Message ordering (mixnet doesn't guarantee order) |
| `message.rs` | Wire protocol for connections/substreams |
| `error.rs` | Error types |

# nym-libp2p-wasm

This crate provides a libp2p `Transport` implementation that uses the Nym mixnet for p2p communication in browser environments.

## Features

- libp2p `Transport` trait implementation
- Stream multiplexing via `StreamMuxer`
- Message ordering over the unordered mixnet
- Anonymous replies using SURBs
- Browser compatible

## Building

```sh
# Check
cargo check -p nym-libp2p-wasm --target wasm32-unknown-unknown
cargo build -p nym-libp2p-wasm --target wasm32-unknown-unknown

# Build with wasm-pack (for browser):
cd wasm/libp2p-nym
wasm-pack build --target web
```

## Testing

Run WASM tests with wasm-pack:
```sh
cd wasm/libp2p-nym

# Browser tests (headless)
wasm-pack test --headless --firefox
wasm-pack test --headless --chrome
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

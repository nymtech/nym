# WebSocket Client (Standalone)

> This client can also be used via the [Rust SDK](../rust) and [Go/C++ FFI](../rust/ffi).

The standalone WebSocket client connects to the Nym Mixnet and exposes a WebSocket interface on localhost. Applications in any language can connect to this WebSocket to send and receive messages through the Mixnet.

This is useful if you're building an application in a language other than TypeScript or Rust and cannot use one of the SDKs directly. Your application connects to the local WebSocket, and the client handles Sphinx packet construction, gateway registration, and key management.

## Download or compile

Pre-built binaries for macOS and Debian-based Linux are available on the [GitHub releases page](https://github.com/nymtech/nym/releases). Look for the `nym-client` binary.

To build from source:

```bash
git clone https://github.com/nymtech/nym.git
cd nym
cargo build --release -p nym-client
```

The binary will be at `target/release/nym-client`.

## Initialize and run

```bash
# Create a new client identity
./nym-client init --id my-client

# Start the client
./nym-client run --id my-client
```

The client prints its Nym address on startup and opens a WebSocket on `ws://127.0.0.1:1977`.

## Sending and receiving

Connect to `ws://127.0.0.1:1977` from your application. Messages are JSON-formatted:

**Send a message:**
```json
{
  "type": "send",
  "message": "hello",
  "recipient": "<recipient-nym-address>"
}
```

**Receive messages:** The client pushes incoming messages to your WebSocket connection as they arrive through the Mixnet.

## Source code

The client source is in the [Nym monorepo](https://github.com/nymtech/nym/tree/master/clients/native).

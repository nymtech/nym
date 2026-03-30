# smolmix-tungstenite

WebSocket connections through the Nym mixnet. Wraps [tokio-tungstenite](https://docs.rs/tokio-tungstenite) with a `connect()` function that handles DNS resolution, TCP, TLS, and the WebSocket upgrade — all through a smolmix `Tunnel`.

## Quick start

```rust
use smolmix_tungstenite::{connect, Message};
use futures::{SinkExt, StreamExt};

let tunnel = smolmix::Tunnel::new().await?;
let (mut ws, _resp) = connect(&tunnel, "wss://echo.websocket.org").await?;

ws.send(Message::Text("hello".into())).await?;
let reply = ws.next().await.ok_or("no reply")??;
println!("{}", reply.into_text()?);
ws.close(None).await?;
```

## API

- **`connect(&tunnel, url)`** — connect to a `wss://` WebSocket server through the mixnet
  - Handles DNS, TCP, TLS, and WebSocket upgrade in one call
  - Returns `(WsStream, Response)` — the stream implements `Sink<Message> + Stream<Item = Message>`
  - Only `wss://` is supported (use raw `TcpStream` for unencrypted WebSocket)

### Re-exports

Commonly-used types are re-exported so you don't need `tokio-tungstenite` or `tungstenite` in your `Cargo.toml`:

- `Message` — the WebSocket message type
- `tungstenite` — the underlying tungstenite crate (for `Error`, `protocol::WebSocketConfig`, etc.)

### Types

- **`WsStream`** — type alias for `WebSocketStream<TlsStream<TcpStream>>`
- **`Error`** — wraps `io::Error` and `tungstenite::Error`

## Example

Clearnet-vs-mixnet WebSocket echo comparison with timing:

```sh
cargo run -p smolmix-tungstenite --example echo
cargo run -p smolmix-tungstenite --example echo -- --ipr <IPR_ADDRESS>
```

## Dependencies

```toml
[dependencies]
smolmix-tungstenite = { workspace = true }
```

This crate depends on `smolmix` and `smolmix-dns` (DNS resolution goes through the tunnel automatically).

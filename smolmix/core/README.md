# smolmix

A TCP/UDP tunnel over the Nym mixnet. Uses smoltcp as a userspace network stack and connects to an Exit Gateway's IP Packet Router, so the exit IP is the gateway's — not yours.

`Tunnel` gives you standard `TcpStream` and `UdpSocket` types (from tokio-smoltcp) that work transparently with the async Rust ecosystem: tokio-rustls for TLS, hyper for HTTP, tokio-tungstenite for WebSockets, etc.

## Quick start

```rust
use smolmix::Tunnel;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

let tunnel = Tunnel::new().await?;

// Raw TCP — works with any protocol
let mut tcp = tunnel.tcp_connect("1.1.1.1:80".parse()?).await?;
tcp.write_all(b"GET / HTTP/1.1\r\nHost: 1.1.1.1\r\nConnection: close\r\n\r\n").await?;

// Raw UDP — datagrams over the mixnet
let udp = tunnel.udp_socket().await?;
udp.send_to(&packet, "1.1.1.1:53".parse()?).await?;
```

## Companion crates

For higher-level protocols, add the companion crates you need:

| Crate | What it provides |
|-------|-----------------|
| **smolmix** | `Tunnel`, `TcpStream`, `UdpSocket` (this crate) |
| [smolmix-dns](../dns/) | `Resolver` newtype wrapping hickory-resolver |
| [smolmix-tls](../tls/) | Shared `TlsConnector` and `connect()` with webpki roots |
| [smolmix-hyper](../hyper/) | `Client` newtype wrapping hyper-util |
| [smolmix-tungstenite](../tungstenite/) | `connect()` for WebSocket over TLS |
| [smolmix-libp2p](../libp2p/) | `SmolmixTransport` implementing libp2p `Transport` |

```toml
[dependencies]
smolmix = { workspace = true }
smolmix-dns = { workspace = true }         # DNS resolution
smolmix-tls = { workspace = true }         # TLS setup (used by hyper + tungstenite)
smolmix-hyper = { workspace = true }       # HTTP client
smolmix-tungstenite = { workspace = true } # WebSocket client
smolmix-libp2p = { workspace = true }     # libp2p transport
```

## Examples

All examples include a clearnet-vs-mixnet comparison with timing and accept `--ipr <ADDRESS>` for targeting a specific exit node.

```sh
cargo run -p smolmix --example tcp         # raw TCP connection
cargo run -p smolmix --example udp         # raw UDP datagram
cargo run -p smolmix --example websocket   # WebSocket over TLS (raw TcpStream composability)
```

## Architecture

See [`src/ARCHITECTURE.md`](src/ARCHITECTURE.md).

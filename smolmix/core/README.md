# smolmix

A TCP/UDP tunnel over the Nym mixnet. Uses smoltcp as a userspace network stack and connects to an Exit Gateway's IP Packet Router, so the exit IP is the gateway's — not yours.

`Tunnel` gives you standard `TcpStream` and `UdpSocket` types (from tokio-smoltcp) that work transparently with the async Rust ecosystem: tokio-rustls for TLS, hyper for HTTP, tokio-tungstenite for WebSockets, etc.

## Examples

All examples include a clearnet-vs-mixnet comparison with timing and accept `--ipr <ADDRESS>` for targeting a specific exit node.

```sh
cargo run -p smolmix --example tcp         # raw TCP connection
cargo run -p smolmix --example udp         # raw UDP datagram
cargo run -p smolmix --example websocket   # WebSocket over TLS (raw TcpStream composability)
```

## Architecture

See [`src/ARCHITECTURE.md`](src/ARCHITECTURE.md).

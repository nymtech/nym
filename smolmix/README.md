# smolmix

A TCP/UDP tunnel over the Nym mixnet. Uses smoltcp as a userspace network stack and connects to an Exit Gateway's IP Packet Router, so the exit IP is the gateway's — not yours.

`Tunnel` gives you standard `TcpStream` and `UdpSocket` types (from tokio-smoltcp) that work transparently with the async Rust ecosystem: tokio-rustls for TLS, hyper for HTTP, tokio-tungstenite for WebSockets, etc.

## Examples

- `tunnel_https` — HTTPS GET via tokio-rustls + hyper over the mixnet
- `tunnel_websocket` — WebSocket connection via tokio-tungstenite over TLS
- `tunnel_dns` — DNS lookup via hickory-proto over UDP

## Architecture

See [`src/ARCHITECTURE.md`](src/ARCHITECTURE.md) (also available on [docs.rs](https://docs.rs/smolmix)).

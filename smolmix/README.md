# smolmix

TCP/UDP tunnel over the Nym mixnet. Uses a userspace network stack (smoltcp)
to provide real `TcpStream` and `UdpSocket` types that work transparently
with the async Rust ecosystem — tokio-rustls, hyper, tokio-tungstenite,
libp2p, and anything else built on `AsyncRead + AsyncWrite`.

## Why IP, not messages

The Nym SDK works at the **message layer**: you send and receive `Vec<u8>`
payloads through the mixnet. Every protocol must be hand-adapted — you need
custom framing, ordering, connection state, and flow control.

`smolmix` operates at the **IP layer**. A userspace smoltcp stack manages
real TCP state machines (retransmits, windowing, port allocation) and UDP
datagram delivery, and the mixnet becomes a transparent transport underneath.
Any protocol that works over TCP or UDP works over smolmix — with zero
adaptation.

```text
┌──────────────────────────────────────────────────────────────────┐
│  Application protocols that "just work" over smolmix             │
│                                                                  │
│  ┌──────────┐ ┌──────────┐ ┌──────────────┐ ┌────────────────┐   │
│  │ TLS      │ │ HTTP/1.1 │ │ WebSocket    │ │ libp2p         │   │
│  │ (rustls) │ │ (hyper)  │ │ (tungstenite)│ │ (noise+yamux)  │   │
│  └────┬─────┘ └────┬─────┘ └──────┬───────┘ └───────┬────────┘   │
│       │             │              │                 │           │
│       └─────────────┴──────────────┴─────────────────┘           │
│                             │                                    │
│                   tokio_smoltcp::TcpStream                       │
│               (AsyncRead + AsyncWrite, Send, Unpin)              │
├──────────────────────────────────────────────────────────────────┤
│                     smolmix Tunnel                               │
│                   (smoltcp → mixnet → IPR)                       │
└──────────────────────────────────────────────────────────────────┘
```

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

## Examples

```sh
cargo run -p smolmix --example tcp         # HTTPS via hyper
cargo run -p smolmix --example udp         # DNS via hickory-proto
cargo run -p smolmix --example websocket   # WebSocket via tungstenite
```

## Architecture

See [`core/src/ARCHITECTURE.md`](core/src/ARCHITECTURE.md) for the internal
stack (smoltcp, device adapter, bridge, mixnet client).

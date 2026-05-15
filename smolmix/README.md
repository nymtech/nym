# smolmix

TCP/UDP tunnel over the Nym mixnet. Uses a userspace network stack (smoltcp)
to provide real `TcpStream` and `UdpSocket` types that work with the async
Rust ecosystem: tokio-rustls, hyper, tokio-tungstenite, libp2p, and anything
else built on `AsyncRead + AsyncWrite`.

## Workspace layout

```text
                  smolmix-hyper
                   (top-level)
                   /          \
                  v            v
            smolmix-dns ←→ smolmix-tls
            (resolution)    (encryption)
                   \          /
                    v        v
                     smolmix
                    (tunnel)
```

`smolmix` provides the underlying TCP/UDP tunnel. `smolmix-dns` and `smolmix-tls` are companion crates that each handle one concern; `smolmix-hyper` glues them into a complete HTTP client. Pick the level of abstraction that matches your needs; the lower-level crates remain useful when you want manual control (e.g. websockets, libp2p, custom protocols).

## Why IP, not messages

The Nym SDK works at the message layer: you send and receive `Vec<u8>`
payloads through the mixnet. Every protocol has to be hand-adapted, with
custom framing, ordering, connection state, and flow control.

`smolmix` operates at the IP layer. A userspace smoltcp stack manages real
TCP state machines (retransmits, windowing, port allocation) and UDP
datagram delivery, and the mixnet becomes the transport underneath. Any
protocol that works over TCP or UDP works over smolmix without adaptation.

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

// Raw TCP, works with any protocol
let mut tcp = tunnel.tcp_connect("1.1.1.1:80".parse()?).await?;
tcp.write_all(b"GET / HTTP/1.1\r\nHost: 1.1.1.1\r\nConnection: close\r\n\r\n").await?;

// Raw UDP, datagrams over the mixnet
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

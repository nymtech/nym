# smolmix

TCP/UDP tunnel over the Nym mixnet. Uses a userspace network stack (smoltcp)
to provide real `TcpStream` and `UdpSocket` types that work transparently
with the async Rust ecosystem — tokio-rustls, hyper, tokio-tungstenite,
libp2p, and anything else built on `AsyncRead + AsyncWrite`.

## Why TCP, not messages

The Nym SDK works at the **message layer**: you send and receive `Vec<u8>`
payloads through the mixnet. Every protocol must be hand-adapted — you need
custom framing, ordering, connection state, and flow control.

smolmix operates at the **TCP layer**. A userspace smoltcp stack manages
real TCP state machines (retransmits, windowing, port allocation), and the
mixnet becomes a transparent transport underneath. Any protocol that works
over a TCP stream works over smolmix — with zero adaptation.

```text
┌──────────────────────────────────────────────────────────────────┐
│  Application protocols that "just work" over smolmix TcpStream   │
│                                                                  │
│  ┌──────────┐ ┌──────────┐ ┌──────────────┐ ┌────────────────┐  │
│  │ TLS      │ │ HTTP/1.1 │ │ WebSocket    │ │ libp2p         │  │
│  │ (rustls) │ │ (hyper)  │ │ (tungstenite)│ │ (noise+yamux)  │  │
│  └────┬─────┘ └────┬─────┘ └──────┬───────┘ └───────┬────────┘  │
│       │             │              │                 │            │
│       └─────────────┴──────────────┴─────────────────┘            │
│                             │                                     │
│                   tokio_smoltcp::TcpStream                        │
│               (AsyncRead + AsyncWrite, Send, Unpin)               │
├──────────────────────────────────────────────────────────────────┤
│                     smolmix Tunnel                                │
│                   (smoltcp → mixnet → IPR)                        │
└──────────────────────────────────────────────────────────────────┘
```

Concrete example: the Nym SDK's libp2p integration (`libp2p_shared/`) is
~2400 lines — custom `Connection` (StreamMuxer), custom `Substream`
(AsyncRead/Write), nonce-based ordering, and a custom handshake. With
smolmix, the same integration is ~175 lines: parse a multiaddr, resolve DNS
through the tunnel, and call `tunnel.tcp_connect()`. libp2p's built-in noise
and yamux work natively because they only need `AsyncRead + AsyncWrite`.

Each companion crate is a thin wrapper — typically under 200 lines — that
adapts an existing library to use tunnel I/O instead of OS sockets. The key
insight is that `tokio_smoltcp::TcpStream` implements tokio's `AsyncRead +
AsyncWrite`, so the only work needed is bridging to whatever I/O trait the
target library expects:

| Crate | Lines | Trait implemented | I/O bridge |
|-------|------:|-------------------|------------|
| `smolmix-dns` | ~110 | hickory `RuntimeProvider` | `AsyncIoTokioAsStd` (tokio→futures) |
| `smolmix-tls` | ~60 | *(none — factory only)* | direct (tokio-rustls takes `AsyncRead/Write`) |
| `smolmix-hyper` | ~170 | `tower::Service<Uri>` | `TokioIo` + `pin_project!` enum |
| `smolmix-tungstenite` | ~140 | *(none — composition)* | direct (tungstenite takes `AsyncRead/Write`) |
| `smolmix-libp2p` | ~175 | libp2p `Transport` | `Compat` (tokio→futures) |

### smolmix-dns

hickory-resolver's extension point is the `RuntimeProvider` trait — it
controls how the resolver creates TCP connections and UDP sockets.
`SmolmixRuntimeProvider` implements this trait, routing all I/O through the
tunnel:

```text
RuntimeProvider::connect_tcp()  →  tunnel.tcp_connect()  →  AsyncIoTokioAsStd<TcpStream>
RuntimeProvider::bind_udp()     →  tunnel.udp_socket()   →  SmolmixUdpSocket (newtype)
```

hickory expects `futures_io::AsyncRead/Write` for TCP, not tokio's version.
`AsyncIoTokioAsStd` (from hickory-proto) adapts between them — and because
hickory's `DnsTcpStream` has a blanket impl for any `futures_io::AsyncRead +
AsyncWrite`, the wrapped stream satisfies it automatically. Zero glue code.

For UDP, `SmolmixUdpSocket` is a thin newtype over `tokio_smoltcp::UdpSocket`
that implements hickory's `DnsUdpSocket` — just delegates `poll_recv_from`
and `poll_send_to`, whose signatures already match.

### smolmix-tls

No trait adaptation needed. `tokio-rustls` works directly with anything that
implements tokio's `AsyncRead + AsyncWrite` — which `TcpStream` does. This
crate is pure configuration: build a `ClientConfig` with webpki roots, wrap
it in a `TlsConnector`, expose `connect()` and `connect_with()`.

Re-exports `TlsStream` and `TlsConnector` so downstream crates don't need
`tokio-rustls` in their Cargo.toml.

### smolmix-hyper

hyper-util's `Client` uses the `tower::Service<Uri>` trait to open
connections. `SmolmixConnector` implements this: given a URI, it resolves the
hostname, connects TCP, and optionally wraps in TLS.

The interesting part is the return type. hyper needs a stream that implements
`hyper::rt::Read + Write + Connection`. `TokioIo<T>` (from hyper-util)
provides this, but it needs a single type — not "sometimes TLS, sometimes
plain". `MaybeTlsStream` solves this with a two-variant enum and
`pin_project_lite` for safe pin projection through `AsyncRead`/`AsyncWrite`:

```text
SmolmixConnector::call(uri)
  → resolve + tcp_connect + optional TLS
  → MaybeTlsStream::Plain { TcpStream }
    or MaybeTlsStream::Tls { TlsStream<TcpStream> }
  → TokioIo<MaybeTlsStream>   (implements hyper's Read/Write/Connection)
```

### smolmix-tungstenite

No new types or trait impls — pure function composition.
`tokio_tungstenite::client_async()` takes any `AsyncRead + AsyncWrite`
stream, and `TlsStream<TcpStream>` qualifies. The `connect()` function
chains four steps:

```text
connect(tunnel, request)
  → smolmix_dns::resolve(host, port)    DNS through tunnel
  → tunnel.tcp_connect(addr)            TCP through mixnet
  → smolmix_tls::connect(tcp, host)     TLS handshake
  → client_async(request, tls_stream)   WebSocket upgrade
```

### smolmix-libp2p

libp2p's extension point is the `Transport` trait. `SmolmixTransport`
implements it for dial-only connections (no inbound — that would require IPR
listener support).

libp2p uses `futures_io::AsyncRead/Write`, not tokio's. The bridge is
`tokio_util::compat::Compat<T>` — called via `.compat()` on the TcpStream.
This is zero-cost trait delegation (no buffering, no copying):

```text
SmolmixTransport::dial(multiaddr)
  → parse /ip4/.../tcp/... or /dns4/.../tcp/...
  → smolmix_dns::resolve() if hostname
  → tunnel.tcp_connect(addr)
  → tcp.compat()   →  Compat<TcpStream>  (futures_io AsyncRead/Write)
```

libp2p's built-in upgrade pipeline — noise (encryption) → yamux
(multiplexing) — works over any `futures_io::AsyncRead + AsyncWrite`, so
the standard `.upgrade(V1).authenticate(noise).multiplex(yamux)` chain
works without modification.

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

## Crates

| Crate | What it provides |
|-------|-----------------|
| [`smolmix`](core/) | `Tunnel`, `TcpStream`, `UdpSocket` |
| [`smolmix-dns`](dns/) | `Resolver` newtype wrapping hickory-resolver |
| [`smolmix-tls`](tls/) | Shared `TlsConnector` and `connect()` with webpki roots |
| [`smolmix-hyper`](hyper/) | `Client` newtype wrapping hyper-util |
| [`smolmix-tungstenite`](tungstenite/) | `connect()` for WebSocket over TLS |
| [`smolmix-libp2p`](libp2p/) | `SmolmixTransport` implementing libp2p `Transport` |

`smolmix-hyper` and `smolmix-tungstenite` depend on `smolmix-dns` (hostname
resolution through the tunnel) and `smolmix-tls` (shared TLS setup).

```toml
[dependencies]
smolmix = { workspace = true }
smolmix-dns = { workspace = true }         # DNS resolution
smolmix-tls = { workspace = true }         # TLS (used by hyper + tungstenite)
smolmix-hyper = { workspace = true }       # HTTP client
smolmix-tungstenite = { workspace = true } # WebSocket client
smolmix-libp2p = { workspace = true }     # libp2p transport
```

## Examples

Each crate has its own examples with clearnet-vs-mixnet comparisons:

```sh
cargo run -p smolmix             --example tcp         # raw TCP
cargo run -p smolmix             --example udp         # raw UDP
cargo run -p smolmix             --example websocket   # WebSocket over TLS
cargo run -p smolmix-dns         --example resolve     # DNS resolution
cargo run -p smolmix-hyper       --example get         # HTTPS GET
cargo run -p smolmix-hyper       --example post        # HTTP POST
cargo run -p smolmix-tungstenite --example echo        # WebSocket echo

# libp2p: start the clearnet listener, then dial through the mixnet
cargo run -p smolmix-libp2p      --example listener
cargo run -p smolmix-libp2p      --example ping -- <MULTIADDR from listener>
```

## Architecture

See [`core/src/ARCHITECTURE.md`](core/src/ARCHITECTURE.md) for the internal
stack (smoltcp, device adapter, bridge, mixnet client).

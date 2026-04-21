# Architecture

smolmix is a TCP/UDP tunnel over the Nym mixnet. It gives you standard
`TcpStream` and `UdpSocket` types that work transparently with the async Rust
ecosystem (tokio-rustls, hyper, tokio-tungstenite, etc.) while routing all
traffic through the mixnet for metadata privacy.

## Stack

```text
┌─────────────────────────────────────────────────────────────────┐
│  User code                                                      │
│  tunnel.tcp_connect() → TcpStream (AsyncRead + AsyncWrite)      │
│  tunnel.udp_socket()  → UdpSocket (send_to / recv_from)         │
├─────────────────────────────────────────────────────────────────┤
│  tokio-smoltcp::Net                                             │
│  Owns the smoltcp Interface + SocketSet + async poll loop.      │
│  Manages TCP state machines, retransmits, port allocation.      │
├─────────────────────────────────────────────────────────────────┤
│  NymAsyncDevice  (device.rs)                                    │
│  Stream + Sink adapter for raw IP packets over mpsc channels.   │
├─────────────────────────────────────────────────────────────────┤
│  NymIprBridge  (bridge.rs)                                      │
│  Background task shuttling packets between channels and the     │
│  mixnet. Bundles outgoing packets with MultiIpPacketCodec       │
│  (required by the IPR protocol).                                │
├─────────────────────────────────────────────────────────────────┤
│  IpMixStream → MixnetClient → Nym mixnet → IPR exit node        │
└─────────────────────────────────────────────────────────────────┘
```

## Data flow

```text
outgoing: smoltcp → NymAsyncDevice (Sink) → channel → NymIprBridge → IpMixStream → mixnet
incoming: mixnet → IpMixStream → NymIprBridge → channel → NymAsyncDevice (Stream) → smoltcp
```

tokio-smoltcp handles all the hard parts (smoltcp polling, TCP state machines,
port allocation, waker management). We just give it a device that produces and
consumes raw IP packets — `NymAsyncDevice` wraps the mpsc channel ends in the
`Stream`/`Sink` traits that tokio-smoltcp requires.

## Key design decisions

- **Single async device adapter.** All traffic flows through one
  `NymAsyncDevice`. If you need a new transport type (e.g. ICMP), add a method
  to `Tunnel` rather than introducing a separate device — the device and bridge
  don't need to change. smoltcp already supports ICMP sockets; you'd enable
  the `socket-icmp` feature in `Cargo.toml`, add a method like
  `Tunnel::icmp_socket()` that calls the appropriate `Net` method, and expose
  the socket type via a re-export in `lib.rs`.

- **Tokio-only.** The bridge, SDK (`IpMixStream`, `MixnetClient`), and shutdown
  signaling are tokio-based. The data-plane channels use `futures::channel::mpsc`
  because `UnboundedSender` implements `Sink` — required by tokio-smoltcp's
  `AsyncDevice` trait. An earlier version had a sync smoltcp `Device` adapter
  for use without tokio-smoltcp, but it still required a tokio runtime
  underneath (for the bridge and SDK), so it provided no real runtime
  independence — just duplicated the bridging logic. If alternative-runtime
  support is ever needed, it would require swapping out the bridge, SDK, and
  channel layers — a separate crate, not a feature flag on this one.

- **Unbounded channels.** The channels between the device and bridge are
  unbounded. Backpressure is handled at the mixnet layer (IPR protocol), not
  at the channel level. If that assumption changes, consider bounded channels
  with a drop policy.

- **`Medium::Ip` (no Ethernet framing).** Raw IP packets go in and out,
  matching what the IPR protocol expects.

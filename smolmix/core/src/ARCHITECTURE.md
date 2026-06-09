# Architecture

smolmix tunnels TCP and UDP over the Nym mixnet. It exposes `TcpStream` and
`UdpSocket` types compatible with the async Rust ecosystem (tokio-rustls,
hyper, tokio-tungstenite), with all traffic routed through the mixnet so a
network observer cannot correlate source and destination.

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

This crate (`smolmix`) provides the underlying TCP/UDP tunnel. The companion crates each handle one concern: [`smolmix-dns`](https://crates.io/crates/smolmix-dns) for tunneled DNS resolution, [`smolmix-tls`](https://crates.io/crates/smolmix-tls) for TLS over the tunnel, and [`smolmix-hyper`](https://crates.io/crates/smolmix-hyper) which combines them into a complete HTTP client. The arrows show conceptual layering, not strict Cargo dependencies.

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

tokio-smoltcp drives the smoltcp poll loop, TCP state machines, port
allocation, and waker management. Its only requirement is a device producing
and consuming raw IP packets. `NymAsyncDevice` satisfies that by wrapping the
mpsc channel ends in the `Stream` and `Sink` traits.

## Key design decisions

### Single async device adapter

All traffic flows through one `NymAsyncDevice`. New transport types (e.g.
ICMP) are added as methods on `Tunnel` rather than as separate devices, so
neither the device nor the bridge needs to change. smoltcp already supports
ICMP sockets: enable the `socket-icmp` feature in `Cargo.toml`, add a method
like `Tunnel::icmp_socket()` that calls the appropriate `Net` method, and
re-export the socket type from `lib.rs`.

### Tokio-only

The bridge, SDK (`IpMixStream`, `MixnetClient`), and shutdown signalling are
tokio-based. The data-plane channels use `futures::channel::mpsc` because
`UnboundedSender` implements `Sink`, which tokio-smoltcp's `AsyncDevice` trait
requires.

An earlier version had a sync smoltcp `Device` adapter for use without
tokio-smoltcp, but it still required a tokio runtime underneath (for the
bridge and SDK), so it provided no real runtime independence and just
duplicated the bridging logic. Supporting alternative runtimes would require
replacing the bridge, SDK, and channel layers, which is the scope of a
separate crate rather than a feature flag on this one.

### Unbounded channels

The channels between the device and bridge are unbounded. Backpressure is
applied at the mixnet layer (the IPR protocol), not at the channel level.
If that assumption changes, switch to bounded channels with a drop policy.

### `Medium::Ip` (no Ethernet framing)

Raw IP packets in and out, matching what the IPR protocol expects.

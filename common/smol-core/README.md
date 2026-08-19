# nym-smol-core

A transport-agnostic, pure-Rust userspace TCP/IP stack. It turns a
bidirectional stream of raw IP packets (`Vec<u8>`) into tokio-native
`TcpStream` / `UdpSocket` sockets plus a tunnel-scoped DNS resolver — with **no
OS `tun` device and no elevated privileges**, and no Go / gVisor / FFI netstack.

It is the shared stack beneath [`smolmix`](../../smolmix) (the 5-hop mixnet
tunnel) and [`nym-smoldvpn`](../../smoldvpn) (the WireGuard dVPN
datapath).

## Concept

The core abstraction is the **IP-packet transport**: anything that can produce
inbound IP packets and consume outbound ones drives the same stack. Provide a
`ChannelDevice` fed from your transport's channels (a mixnet bridge, a WireGuard
datapath, a test harness), build a `Stack`, and open sockets.

```rust
use futures::channel::mpsc;
use nym_smol_core::{ChannelDevice, Stack, StackConfig, DEFAULT_MTU};

let (outbound_tx, outbound_rx) = mpsc::unbounded::<Vec<u8>>(); // stack -> transport
let (inbound_tx, inbound_rx) = mpsc::unbounded::<Vec<u8>>();   // transport -> stack

let device = ChannelDevice::new(inbound_rx, outbound_tx, DEFAULT_MTU);
let stack = Stack::new(device, StackConfig::new("10.0.0.2".parse()?));

let tcp = stack.tcp_connect("1.1.1.1:443".parse()?).await?; // AsyncRead + AsyncWrite
let udp = stack.udp_socket().await?;
let addrs = stack.resolve("example.com").await?;            // DNS over the tunnel
```

## API surface

- `Stack` — `tcp_connect`, `udp_socket` / `udp_socket_on`, `resolve`,
  `tcp_connect_host`, `net()` (advanced), `with_dns_config`.
- `StackConfig` — assigned IPv4 (`/32`), optional IPv6, MTU.
- `ChannelDevice` — the `tokio-smoltcp` `AsyncDevice` over `Vec<u8>` channels.
- `DnsConfig` — upstream server + timeout for in-tunnel resolution.

## Tests

`cargo test -p nym-smol-core` runs the stack integration tests (two crossed stacks):
UDP round-trip, TCP round-trip, connect-failure, and DNS resolution over a stack
socket.

## Design

See the architecture docs in
[`docs/design/smoldvpn/`](../../docs/design/smoldvpn/) and the
[`smol-core-stack`](../../openspec/specs/smol-core-stack/spec.md) OpenSpec capability.

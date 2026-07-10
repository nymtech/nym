# nym-smol-dvpn

A pure-Rust, userspace **1-/2-hop WireGuard dVPN datapath** built on
[`boringtun`](https://docs.rs/boringtun) and [`smol-core`](../../../common/smol-core),
with **no OS `tun` device and no root**. Application traffic flows through the
tunnel via ordinary tokio socket surfaces (`TcpStream`, `UdpSocket`, and
`tonic`/`hyper` connectors).

## Data-plane modes

Three modes (design D5), selected on the builder:

- **one-hop** — a single `boringtun` `Tunn` to one gateway.
- **two-hop** — nested `Tunn`s: the exit tunnel's ciphertext is framed as an
  inner IP/UDP datagram (via `smoltcp::wire`) and re-encrypted by the entry
  tunnel (design D4, proven in conformance spike A).
- **QUIC-tunnelling two-hop** — the entry leg is fronted by an inline QUIC
  bridge (ALPN `hq-29`, ed25519-SPKI pinning, 2-byte length framing) for
  clients blocked from pure UDP. QUIC only ever fronts the two-hop entry leg.

## Usage

The datapath is **decoupled from provisioning**: build a `PeerConfig` per hop
(e.g. by mapping a `nym-sdk-session` registration) and hand it to a
`TunnelBuilder`.

```rust
use nym_smol_dvpn::{TunnelBuilder, PeerConfig, BridgeParams};

// Two-hop over direct UDP:
let tunnel = TunnelBuilder::two_hop(entry, exit)
    .cancellation_token(token)
    .connect()
    .await?;

let mut tcp = tunnel.tcp_connect("1.1.1.1:443".parse()?).await?;
// ... use `tcp` as any AsyncRead + AsyncWrite ...

// gRPC through the tunnel:
let channel = tonic::transport::Endpoint::from_static("http://10.0.0.1:50051")
    .connect_with_connector(tunnel.connector())
    .await?;

tunnel.shutdown().await;

// QUIC-bridged two-hop:
let tunnel = TunnelBuilder::two_hop(entry, exit)
    .quic_bridge(BridgeParams { addresses, sni_host, id_pubkey })
    .connect()
    .await?;
```

## Examples

Runnable end-to-end demos live in `examples/` (all need a funded `MNEMONIC` and
a live Nym network; shared setup is in `examples/common/`):

- `smol-dvpn-config` — export a plain WireGuard config from a single-hop
  registration.
- `smol-dvpn-topup` — spend a stored ticket and report updated bandwidth.
- `smol-dvpn-grpc` — a `tonic` gRPC health check through the tunnel.
- `two-hop-ip` — prove a two-hop tunnel relocates your public IP (queries
  `ipinfo.io` directly, then through the tunnel). Pass `-- --quic` to require a
  QUIC-bridge entry gateway.
- `two-hop-quic` — a two-hop tunnel whose **entry leg is carried over a QUIC
  bridge** (for clients blocked from plain WireGuard/UDP).
- `zcash-sync` — time syncing the last 1000 Zcash compact blocks from a public
  `lightwalletd` (gRPC-over-TLS) directly vs. through the tunnel. Also accepts
  `-- --quic`.

```sh
MNEMONIC="<funded mnemonic>" cargo run -p nym-smol-dvpn --example two-hop-quic
```

QUIC-entry selection (`two-hop-quic`, `--quic`) needs a dVPN gateway-directory
URL so the session can discover QUIC-bridge-capable gateways and their bridge
params; the examples default to the sandbox directory (override with
`DVPN_DIRECTORY_URL`). If no QUIC-capable entry matches the requested
country/identity, selection fails with `NoQuicGateway`.

## Features

- `CancellationToken` aborts setup or tears down the long-lived tunnel;
  `shutdown()` is equivalent. Issued tickets are never touched by this crate.
- Configurable per-hop MTU (reference defaults: overhead 80/hop; desktop
  1420/1340; mobile 1360/1280).
- DNS-in-tunnel by default (configurable), via the `smol-core` resolver.
- boringtun timer pump on the datapath task; keepalive/handshake/rekey routed
  through the active transport.
- Optional `SocketProtector` callback (Linux/Android) for the egress UDP socket.

## Third-party dependencies

`boringtun` (BSD-3-Clause), `quinn` + `quinn-proto` (MIT/Apache-2.0) are declared
**crate-local** here, not promoted to the workspace dependency table (design D10).

## Tests

`cargo test -p nym-smol-dvpn` includes the QUIC bridge conformance test (framing
+ ed25519-SPKI pinning, positive and negative) against a local mock bridge.
End-to-end tunnel bring-up against a live Nym gateway is validated separately
(needs credentials + network).

## Design

See `sdk/rust/docs/nym-sdk-dvpn/` and the `dvpn-tunnel` / `dvpn-quic-bridge`
OpenSpec capabilities.

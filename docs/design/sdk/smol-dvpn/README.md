# `smol-dvpn` — architecture & design

This directory documents the architecture of the pure-Rust, userspace two-hop
WireGuard dVPN datapath. It is background for contributors and integrators who
want to understand *how* the pieces fit together.

> **Building on top of `smol-dvpn`?** Start with the crate README at
> [`sdk/rust/smol-dvpn/README.md`](../../../../sdk/rust/smol-dvpn/README.md) — it
> covers the public API, examples, and how to run against a live network. This
> document is the design rationale behind that API.

## Names at a glance

- **`nym-smol-dvpn`** — the dVPN datapath crate. Directory: `sdk/rust/smol-dvpn/`.
- **`nym-sdk-session`** — the shared provisioning facade crate (ticketbooks +
  gateway registration), used by both mixnet and dvpn modes. In `sdk/rust`.
- **`smol-core`** — the transport-agnostic smoltcp stack. In `common/`.

## One-paragraph summary

`nym-smol-dvpn` brings up a **userspace, two-hop (or single-hop) WireGuard
tunnel** from an entry gateway to an exit gateway using
[`boringtun`](https://github.com/cloudflare/boringtun), with **no OS `tun` device
and no root**. Traffic goes in via ordinary `tokio` primitives — a `TcpStream`, a
`UdpSocket`, or an `AsyncRead + AsyncWrite` — so `tonic`, `hyper`, `reqwest`, and
raw sockets work inside the tunnel unchanged. The caller provides a mnemonic that
funds zk-nym ticketbooks (to pay the gateways for dVPN registration); ticketbooks
are persisted in a credential store so the tunnel can be brought up and down at
will. Everything is `tokio`-async and driven by a `CancellationToken` for aborting
setup or tearing down a long-lived tunnel.

## What is new vs. reused

Most of the machinery already existed in the monorepo; the new work is narrow.

| Capability | Status | Where it lives |
|---|---|---|
| Mnemonic → deposit NYM → signers → issue zk-nym ticketbook | reuse | `nym-bandwidth-fetcher` + `nym-bandwidth-controller` |
| Persistent credential store + auto-refill loop | reuse | `nym-credential-storage` (sqlite) |
| `V1WireguardEntry` / `V1WireguardExit` ticket types | reuse | `nym-credentials-interface` |
| Two-hop entry→exit WG **registration** (LP + mixnet), returns 2× `WireguardConfiguration` (pubkey + PSK + endpoint + assigned IPs) | reuse | `nym-registration-client` |
| Gateway directory fetch | reuse | `nym-validator-client` + `nym-client-core` |
| Gateway selection by identity / random | reuse | `nym-client-core` `init/helpers.rs` |
| Gateway selection by two-letter country code | thin new | filter on described-node `location: Option<celes::Country>` |
| smoltcp userspace TCP/IP stack → `TcpStream` / `UdpSocket` / DNS | reuse | `smol-core` (extracted from `smolmix`) |
| **boringtun single-/two-hop WG datapath** | **new** | `nym-smol-dvpn` |
| **QUIC-bridge transport** for blocked clients | **new** | `nym-smol-dvpn` (over the `nym-bridges` client) |
| Connector adapters (tonic / hyper / reqwest) | **new** | `nym-smol-dvpn` |
| Lifecycle facade + cancellation | **new** | `nym-smol-dvpn` |

## The crate family

```
common/smol-core                 smoltcp stack: channels<IP packet> → TcpStream / UdpSocket / DNS.
                                 Pure tokio + Rust, WASM-capable, transport-agnostic.
    ├── smolmix                  + IPR / mixnet bridge         (5-hop)
    └── sdk/rust/smol-dvpn/      crate nym-smol-dvpn
        (nym-smol-dvpn)          + boringtun WG datapath       (1- or 2-hop)
                                 + GatewayTransport::{ Direct | QuicBridge }  (quinn; crate-local)

sdk/rust/nym-sdk-session         ticketbooks (nym-bandwidth-controller) + gateway registration
                                 (nym-registration-client). Shared by BOTH mixnet and dvpn modes.
```

`nym-smol-dvpn` depends on `nym-sdk-session` for "get me paid access to these
gateways" and on `smol-core` for "give me sockets over this IP-packet pipe". Its
only unique job is the boringtun WireGuard datapath and the transport strategy.

## Contents

- [`design.md`](./design.md) — the full architecture: layering, data-plane
  nesting, control/data seams, transports & the QUIC bridge, credentials, gateway
  selection, DNS, lifecycle, and the public API.

## Specifications

The normative behaviour lives in the OpenSpec capability specs, one per concern:

- [`dvpn-tunnel`](../../../../openspec/specs/dvpn-tunnel/spec.md) — the `nym-smol-dvpn`
  userspace WireGuard datapath, tunnel modes, lifecycle, DNS, MTU, and top-up.
- [`dvpn-quic-bridge`](../../../../openspec/specs/dvpn-quic-bridge/spec.md) — the
  `WgPacketTransport` abstraction and the QUIC bridge transport.
- [`dvpn-tools`](../../../../openspec/specs/dvpn-tools/spec.md) — the example CLIs
  (config export, bandwidth top-up, gRPC/IP/Zcash demos).
- [`dvpn-session`](../../../../openspec/specs/dvpn-session/spec.md) — `nym-sdk-session`
  provisioning: ticketbook issuance, credential storage, gateway selection/registration.
- [`smol-core-stack`](../../../../openspec/specs/smol-core-stack/spec.md) — `smol-core`,
  the transport-agnostic userspace TCP/IP stack.

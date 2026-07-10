# `smol-dvpn` — a pure-Rust, userspace two-hop WireGuard dVPN SDK

> **Status:** design / exploration. No implementation yet. This directory captures
> the architecture agreed during exploration so it can be turned into an
> implementation plan (e.g. an OpenSpec change).

## Names at a glance

- **`nym-smol-dvpn`** — the dVPN datapath crate. Directory: `sdk/rust/smol-dvpn/`.
- **`nym-sdk-session`** — the shared provisioning facade crate (ticketbooks +
  gateway registration), used by both mixnet and dvpn modes. In `sdk/rust`.
- **`smol-core`** — the transport-agnostic smoltcp stack. In `common/`.

## One-paragraph summary

`nym-smol-dvpn` is a new crate in `sdk/rust/smol-dvpn/` that brings up a
**userspace, two-hop (or single-hop) WireGuard tunnel** from an entry gateway to an exit gateway using
[`boringtun`](https://github.com/cloudflare/boringtun), with **no OS `tun` device
and no root**. Traffic goes in via ordinary `tokio` primitives — a `TcpStream`, a
`UdpSocket`, or an `AsyncRead + AsyncWrite` — so `tonic`, `hyper`, `reqwest`, and
raw sockets work inside the tunnel unchanged. The caller provides a mnemonic that
funds zk-nym ticketbooks (to pay the gateways for dVPN registration); ticketbooks
are persisted in a credential store so the tunnel can be brought up and down at
will. Everything is `tokio`-async and driven by a `CancellationToken` for aborting
setup or tearing down a long-lived tunnel.

## What is genuinely new vs. reused

Most of the machinery already exists in the monorepo. The new work is narrow.

| Capability | Status | Where it lives today |
|---|---|---|
| Mnemonic → deposit NYM → signers → issue zk-nym ticketbook | **reuse** | `nym-bandwidth-fetcher` + `nym-bandwidth-controller` |
| Persistent credential store + auto-refill loop | **reuse** | `nym-credential-storage` (sqlite) |
| `V1WireguardEntry` / `V1WireguardExit` ticket types | **reuse** | `nym-credentials-interface` |
| Two-hop entry→exit WG **registration** (LP + mixnet), returns 2× `WireguardConfiguration` (pubkey + PSK + endpoint + assigned IPs) | **reuse** | `nym-registration-client` |
| Gateway directory fetch | **reuse** | `nym-validator-client` + `nym-client-core` |
| Gateway selection by identity / random | **reuse** | `nym-client-core` `init/helpers.rs` |
| Gateway selection by two-letter country code | **thin new** | filter on described-node `location: Option<celes::Country>` |
| smoltcp userspace TCP/IP stack → `TcpStream` / `UdpSocket` / DNS | **reuse pattern** | `smolmix` (today over the 5-hop mixnet) |
| **boringtun single-/two-hop WG datapath** | **NEW** | — (client datapath currently lives out-of-tree in `nym-vpn-client`) |
| **QUIC-bridge transport** for blocked clients | **NEW** | bridge server is deployed out-of-tree; client side is new |
| Connector adapters (tonic / hyper / reqwest) | **NEW, small** | — |
| Lifecycle facade + cancellation | **NEW, small** | — |

## The crate family

```
common/smol-core                 smoltcp stack: channels<IP packet> → TcpStream / UdpSocket / DNS.
                                 Pure tokio + Rust, WASM-capable, transport-agnostic.
    ├── smolmix                  + IPR / mixnet bridge         (5-hop)  [exists; refactor onto smol-core]
    └── sdk/rust/smol-dvpn/      crate nym-smol-dvpn
        (nym-smol-dvpn)          + boringtun WG datapath       (1- or 2-hop)  [NEW]
                                 + GatewayTransport::{ Direct | QuicBridge }  (quinn; crate-local)

sdk/rust/nym-sdk-session         ticketbooks (nym-bandwidth-controller) + gateway registration
                                 (nym-registration-client). Shared by BOTH mixnet and dvpn modes. [NEW]
```

`nym-smol-dvpn` depends on `nym-sdk-session` for "get me paid access to these
gateways" and on `smol-core` for "give me sockets over this IP-packet pipe". Its
only unique job is the boringtun WireGuard datapath and the transport strategy.

## Files in this directory

- [`design.md`](./design.md) — the full architecture: layering, data-plane nesting,
  control/data seams, transports & the QUIC bridge, credentials, gateway
  selection, DNS, lifecycle, and a public API sketch.
- [`open-questions.md`](./open-questions.md) — the two de-risking spikes, naming
  decisions, and tracked risks.

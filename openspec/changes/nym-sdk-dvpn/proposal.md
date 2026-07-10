## Why

NymVPN's userspace two-hop WireGuard dVPN datapath lives out-of-tree in the closed
`nym-vpn-client` repo and is built on Go (`wireguard-go` + a gVisor netstack). There
is no reusable, pure-Rust building block that lets a Nym-built or third-party SDK
consumer bring up a paid dVPN tunnel and push ordinary `tokio` traffic (TCP/UDP,
`tonic`/`hyper`/`reqwest`) through it without root or an OS `tun` device. This change
adds that building block to the Rust SDK, reusing the existing credential,
registration, and stack machinery already in this monorepo.

## What Changes

- Add **`common/smol-core`**: a transport-agnostic, pure-Rust smoltcp stack that
  turns a bidirectional stream of IP-packet `Vec<u8>` into `TcpStream` / `UdpSocket`
  sockets plus a tunnel-scoped DNS resolver. Extracted by refactoring the existing
  **`smolmix`** crate (5-hop mixnet) onto this shared core. **BREAKING** for
  `smolmix` internals only; its public API is preserved.
- Add **`sdk/rust/nym-sdk-session`** (crate `nym-sdk-session`): a provisioning facade
  over `nym-registration-client` + `nym-bandwidth-controller` + the credential store.
  From a caller-supplied mnemonic it issues and persists zk-nym ticketbooks (deposit
  NYM → Nym API signers), selects gateways (by identity / two-letter country code /
  random), and registers gateways. Shared by both mixnet and dvpn modes.
- Add **`sdk/rust/smol-dvpn`** (crate `nym-smol-dvpn`): a `boringtun`-based userspace
  1-/2-hop WireGuard datapath on `smol-core`, provisioned via `nym-sdk-session`. It
  exposes `tokio` socket surfaces (`TcpStream`/`UdpSocket`/`AsyncRead+AsyncWrite` +
  `tonic`/`hyper`/`reqwest` connectors) so application traffic flows inside the
  tunnel. A `CancellationToken` aborts setup or tears down the long-lived tunnel.
  Three data-plane modes: one-hop, two-hop, and QUIC-tunnelling two-hop.
- Add a **QUIC bridge transport** to `nym-smol-dvpn` for clients blocked from pure
  WireGuard/UDP, reimplemented inline (mirroring `nym-vpn-client`'s `transports/`,
  **not** depending on the unstable `nym_bridges` crate): ALPN `hq-29`, ed25519-SPKI
  cert pinning, 2-byte length framing over one reliable QUIC bi-stream. LP
  registration stays **Direct-only** (not bridgeable over QUIC yet).
- Add example programs in `smol-dvpn`: **`smol-dvpn-config`** (single-hop plain
  WireGuard config export), **`smol-dvpn-topup`** (bandwidth top-up via the
  gateway `metadata` endpoint), **`smol-dvpn-grpc`** (`tonic` gRPC health check
  through the tunnel), **`two-hop-ip`** and **`two-hop-quic`** (public-IP
  relocation over a Direct / QUIC-fronted tunnel), and **`zcash-sync`** (Zcash
  compact-block gRPC sync benchmarked direct vs. through the tunnel). The
  configurable examples share a CLI (`--one-hop`/`--two-hop`,
  `--entry`/`--exit`/`--gateway <spec>`, `--quic`).
- New dependencies (`boringtun` BSD-3-Clause, `quinn`, `quinn-proto`) are declared in
  `smol-dvpn`'s own `Cargo.toml`, **not** promoted to the workspace dependency table.

## Capabilities

### New Capabilities
- `smol-core-stack`: pure-Rust userspace smoltcp TCP/IP stack that exposes tokio
  `TcpStream`/`UdpSocket` and a tunnel DNS resolver over an abstract IP-packet
  transport; `smolmix` refactored onto it.
- `dvpn-session`: mnemonic-funded zk-nym ticketbook issuance + persistent credential
  storage, gateway selection (identity / country / random), and gateway registration,
  as a shared `nym-sdk-session` facade; optionally sources the dVPN gateway directory
  for gateway monikers and QUIC-bridge entry selection.
- `dvpn-tunnel`: userspace 1-/2-hop WireGuard datapath (`boringtun`), tunnel lifecycle
  and cancellation, configurable/dynamic MTU, DNS-in-tunnel, and the tokio socket +
  `tonic`/`hyper`/`reqwest` traffic surfaces.
- `dvpn-quic-bridge`: QUIC bridge data-plane transport (inline reimplementation) as an
  alternative to direct UDP for censored clients.
- `dvpn-tools`: example programs — `smol-dvpn-config` (WireGuard config export),
  `smol-dvpn-topup` (metadata bandwidth top-up), `smol-dvpn-grpc` (gRPC through the
  tunnel), `two-hop-ip` / `two-hop-quic` (public-IP relocation), and `zcash-sync`
  (Zcash compact-block sync benchmark), sharing a gateway/hop/QUIC selection CLI.

### Modified Capabilities
<!-- None. `smolmix` is refactored internally but has no existing OpenSpec capability. -->

## Impact

- **New crates:** `common/smol-core`, `sdk/rust/nym-sdk-session`,
  `sdk/rust/smol-dvpn` (`nym-smol-dvpn`). Workspace `members` updated.
- **Refactored crate:** `smolmix` (`smolmix/core`) rebased onto `smol-core`; public
  API preserved (it is published as `1.21.3`).
- **Reused (unchanged):** `nym-registration-client`, `nym-bandwidth-controller`,
  `nym-bandwidth-fetcher`, `nym-credential-storage`, `nym-credentials-interface`,
  `nym-registration-common`, `nym-lp`, `nym-validator-client`, `nym-client-core`,
  `nym-wireguard-private-metadata`.
- **New third-party deps (crate-local to `smol-dvpn`):** `boringtun` (BSD-3-Clause,
  compatible with the Apache-2.0 workspace), `quinn`, `quinn-proto`.
- **Reference-only (not a dependency):** `nym-vpn-client`, `nym-bridges`.
- **Platform:** native only in v1. WASM is an explicit design goal that shapes the
  abstractions (pure-Rust engine, transport seam) but is deferred, not implemented.

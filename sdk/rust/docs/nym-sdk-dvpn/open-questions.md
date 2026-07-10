# `smol-dvpn` — open questions, spikes & decisions

## De-risking spikes (both RESOLVED by reference study)

Both original unknowns were answered by reading `nym-vpn-client` and
`nym-bridges` (see design.md §14). They now become implementation-conformance
tasks, not open research.

### Spike A — two-hop WG datapath addressing → RESOLVED
The exit hop's `WireguardConfiguration.endpoint` is the exit gateway's **real
public address**, and the entry tunnel routes the inner-tunnel UDP to it
(reference: allowed-ips routing in OS-tun mode; an in-tunnel loopback UDP proxy in
userspace mode). Our boringtun model (design.md §4.1) is correct. Concrete params
captured: fixed exit source port (`client_port`, default **54001**); WG overhead
**80 B/hop** (subtract twice for the exit MTU).
**Remaining task:** build a throwaway that registers two hops and pushes one
hand-framed packet through boringtun to confirm parity with the reference.

### Spike B — QUIC bridge relay protocol → RESOLVED
`nym-bridges` is a transparent fixed-target forwarder: **no client-sent
gateway-selection handshake** (bridge is 1:1 with a gateway); `quinn` 0.11; ALPN
`hq-29`; WG packets **2-byte-big-endian length-framed over one reliable bi-stream**
(not RFC 9221 datagrams); ed25519-SPKI cert pinning. Client params
(`ClientOptions { addresses, host, id_pubkey }`) come from the gateway directory /
VPN API. The `nym_bridges` crate is reusable client-side (`transport::quic`,
`certs::IdentityBasedVerifier`, `connection::process_udp`).
**Remaining task:** depend on / mirror `nym_bridges` at a pinned commit (protocol
`version = "0"`, unstable) and conformance-test one WG packet round-trip.

## Decisions made during exploration

- **Datapath library:** `boringtun` (pure-Rust, userspace, no `tun`, no root). **Not**
  `defguard_wireguard_rs` (creates an OS `tun` interface, needs privileges) — even
  though it is what the rest of the repo uses. Confirmed by the "tokio socket, no
  root, WASM-forward" requirements.
- **No Go / gVisor "netstack".** Pure `tokio` + Rust (smoltcp), to keep a future WASM
  build possible.
- **Crate family:** Option B — extract `common/smol-core` (transport-agnostic smoltcp
  stack) and refactor the existing `smolmix` (5-hop) onto it; `smol-dvpn` (2-hop) sits
  beside it. Chosen over two independent siblings to avoid duplicating the smoltcp
  wiring, and it fits the `smol*` naming family.
- **Provisioning is its own `sdk/rust` crate**, not folded into `smol-dvpn`, because
  ticketbooks + gateway registration are shared by mixnet **and** dvpn modes.
- **`boringtun` and `quinn` are dependencies of `smol-dvpn` only** — kept out of the
  workspace-wide dependency graph.
- **New deps declared in the crate manifest, not the workspace table.** Every new
  dependency `nym-smol-dvpn` needs (`boringtun`, `quinn`, `quinn-proto`,
  `nym_bridges` git-pinned, etc.) is declared directly in
  `sdk/rust/smol-dvpn/Cargo.toml`, not promoted to workspace `[workspace.dependencies]`.
  Deps already in the workspace table stay `workspace = true`.
- **LP registration is currently `Direct` only** (not bridgeable over QUIC yet).
  Register directly, then bring the WG data plane up over the bridge.
  `GatewayTransport::QuicBridge` affects the data plane only. LP-over-QUIC is a
  future enhancement (the `LpTransportChannel` trait already permits it).
- **DNS in the tunnel is the default**, configurable.
- **Tunnel is long-lived**, closed explicitly via the `CancellationToken`.
- **~~QUIC datagrams (RFC 9221)~~ → reliable length-framed bi-stream.** Superseded:
  the `nym-bridges` server implements only 2-byte-big-endian length framing over a
  single reliable QUIC bi-stream. The bridge dictates the wire format, so the client
  conforms (no datagram option exists server-side).
- **Three data-plane modes (settled).** The data plane sends in exactly three ways:
  **(1) one-hop**, **(2) two-hop**, **(3) QUIC-tunnelling two-hop**. QUIC bridging is
  scoped to the two-hop path (the bridge is 1:1 with the entry gateway); there is no
  "QUIC one-hop" mode.
- **CLI #1 (`smol-dvpn-config`) is single-hop** by nature; two-hop needs chained wg
  clients. Single-hop is also a first-class supported VPN mode.

## Naming (settled)

- **Provisioning facade crate: `nym-sdk-session`** (in `sdk/rust`). Wraps
  `nym-registration-client` + `nym-bandwidth-controller` + credential store; shared
  by mixnet and dvpn modes.
- **Datapath crate: `nym-smol-dvpn`**, in directory `sdk/rust/smol-dvpn/`. (The
  directory is `smol-dvpn`; the crate/package name is `nym-smol-dvpn`.)
- **Shared smoltcp stack: `smol-core`**, in `common/`.

## Single-hop (settled)

- Single-hop is supported as a mode where the caller specifies **just
  `gateway=…`** instead of `entry=…`/`exit=…` — either via `TunnelMode::SingleHop
  { gateway }` or a dedicated `smol-dvpn-config --gateway <spec>` example.

## WASM (settled)

- **WASM support is a design goal** — it shapes the abstractions now (pure-Rust
  engine, the `WgPacketTransport` seam) — but **is not implemented in v1**; it is
  future work. Raw UDP is unavailable in browsers, so only the `QuicBridge` path
  (over WebTransport) is viable there.

## Tracked risks / things to confirm (resolutions folded in)

- **Gateway-spec surface + single-hop mapping (resolved).** Single-hop: `gateway=…`.
  Two-hop: `entry=…` `exit=…`. `nym-registration-client`'s `BuilderConfig` mandates
  **both** `entry_node` and `exit_node`, and `nym-vpn-client` only runs WireGuard
  two-hop — so single-hop is a new path: register a **single gateway** via the LP
  single-gateway `register_dvpn` path (one `Tunn`, no nesting), rather than forcing
  `entry == exit`.
- **boringtun timers (resolved → design).** Drive `Tunn::update_timers` from a
  **dedicated cancellable background task** (part of the Connected-phase task set),
  routing its output (keepalive/handshake/rekey) through the active
  `WgPacketTransport`. Cancelled by the same token that tears the tunnel down.
- **MTU (resolved → configurable).** Reference values: `WG_TUNNEL_OVERHEAD = 80`
  B/hop; desktop `ENTRY_MTU = 1420` / `EXIT_MTU = 1340`; mobile `ENTRY_MTU = 1360` /
  `EXIT_MTU = 1280`. (Not 1440 — close.) **MTU must be configurable via the SDK
  config and changeable dynamically while the tunnel is up** (re-size the smoltcp
  interface + re-derive per-hop MTUs at runtime), defaulting to the reference values.
- **`nym_bridges` protocol instability (resolved).** Do **not** depend on the
  `nym_bridges` crate — **reimplement the QUIC client inline**, exactly as
  `nym-vpn-client` does (it declares `quinn` directly and has its own
  `transports/mod.rs` + `certs.rs`, no `nym_bridges` dep). Byte-match the three
  invariants (ALPN `hq-29`, ed25519-SPKI verifier, 2-byte length framing); use the
  `nym_bridges` source as a reference to track. `id_pubkey` is base64 of the ed25519
  **public** key (the server's own config uses the secret key — don't conflate).
- **WASM (design goal, deferred).** WASM support **is a design goal**, shaping the
  abstractions now (transport seam, pure-Rust engine), but **will not be implemented
  in v1** — it's future work. In WASM, `Direct` (raw UDP) is unavailable; only the
  `QuicBridge` path (over WebTransport) is viable.
- **Licensing (resolved — OK).** nym is **Apache-2.0**; `boringtun` 0.7.1 is
  **BSD-3-Clause**; `quinn` is **MIT OR Apache-2.0**. BSD-3-Clause is permissive and
  one-way compatible with Apache-2.0 — an Apache-2.0 crate may depend on and
  distribute boringtun, retaining its copyright/license notice. No copyleft concern.
- **`smolmix` is a published crate (`1.21.3`).** Refactoring it onto `smol-core` is a
  backward-compatibility consideration; keep its public API stable.

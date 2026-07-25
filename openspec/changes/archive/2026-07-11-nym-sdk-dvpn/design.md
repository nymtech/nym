## Context

NymVPN's userspace two-hop WireGuard datapath lives out-of-tree (`nym-vpn-client`)
and runs on `wireguard-go` + a gVisor netstack (Go). This repo already has the hard
parts of the *control* path: zk-nym ticketbook issuance/storage
(`nym-bandwidth-controller`/`-fetcher`, `nym-credential-storage`), two-hop gateway
registration (`nym-registration-client`, returning per-hop `WireguardConfiguration`
with gateway pubkey + LP-negotiated PSK), gateway directory/selection
(`nym-client-core`), and a pure-Rust smoltcp stack pattern (`smolmix`, over the 5-hop
mixnet). What is missing is a pure-Rust *client-side WireGuard datapath* that
terminates in ordinary `tokio` sockets.

A full exploration (grounded in this repo plus the `nym-vpn-client` and `nym-bridges`
reference repos) is captured at `docs/design/sdk/smol-dvpn/` (README, design). This
document summarizes the decisions that drive implementation.

## Goals / Non-Goals

**Goals:**
- A pure-Rust, userspace WireGuard dVPN tunnel — **no OS `tun`, no root** — usable as
  an SDK building block by Nym and third parties.
- Single-hop (`gateway=…`) and two-hop (`entry=…`/`exit=…`) modes; gateway selection
  by identity key, two-letter country code, or random.
- Traffic in via ordinary `tokio` primitives: `TcpStream`, `UdpSocket`,
  `AsyncRead+AsyncWrite`, and `tonic`/`hyper`/`reqwest` connectors.
- Mnemonic-funded zk-nym ticketbooks, persisted, so the tunnel comes up/down at will.
- `CancellationToken` to abort the (slow) setup phase or tear down the long-lived
  tunnel.
- Optional QUIC-bridge transport for clients blocked from pure WireGuard/UDP.
- Abstractions shaped so a future WASM build is possible.

**Non-Goals:**
- Not a full VPN client: no OS routing, default-route capture, or kill-switch.
- No Go / gVisor netstack — pure `tokio` + Rust only.
- The QUIC **bridge server** is out of scope (deployed independently).
- WASM is **not implemented** in v1 (design goal only).
- LP registration over QUIC is out of scope for now (registration stays Direct).

## Decisions

**D1 — WireGuard engine: `boringtun`, not `defguard_wireguard_rs` or `wireguard-go`.**
`boringtun` encrypts/decrypts in-process on byte buffers with no OS interface, which
is the only way to satisfy "tokio socket, no root". `defguard` (used by this repo's
gateway side) creates a privileged OS `tun`; `wireguard-go` (the `nym-vpn-client`
engine) is Go + gVisor — exactly what we avoid. `boringtun` is BSD-3-Clause
(compatible with the Apache-2.0 workspace) and WASM-capable.

**D2 — Reuse the smoltcp stack via a shared `smol-core` crate (not copy).** Extract
`smolmix`'s transport-agnostic core (IP-packet `Vec<u8>` stream → `TcpStream` /
`UdpSocket` / DNS) into `common/smol-core`; refactor `smolmix` (5-hop) onto it and add
`nym-smol-dvpn` (2-hop) beside it. Avoids duplicating the smoltcp wiring and fits the
`smol*` family. Alternative (independent sibling crates) rejected for duplication.

**D3 — Shared provisioning facade `nym-sdk-session` is its own crate.** Ticketbooks +
gateway registration are needed by both mixnet and dvpn modes, so this sits beside
the mixnet client rather than inside `nym-smol-dvpn`.

**D4 — Two-hop nesting via userspace double-encapsulation.** The topology,
fixed-port semantics and MTU constants are confirmed against `nym-vpn-client`,
but note the reference datapath runs on **wireguard-go + gVisor (Go/FFI)** and
performs the inner framing inside Go — `boringtun` appears in the reference only
in an offline diagnostic. Our datapath therefore *re-implements* that topology in
pure Rust: the exit `Tunn` encrypts to the exit gateway's real endpoint; that
ciphertext is framed as an IP/UDP packet (via `smoltcp::wire`) and fed to the entry
`Tunn`, which encrypts to the entry gateway. Fixed exit source port bound
dynamically with ref-default 54001 fallback. No loopback proxy needed since we own
both `Tunn`s. Single-hop collapses the inner tunnel. Proven end-to-end during the
conformance-spike phase (task 1.1).

**D5 — Transport seam `WgPacketTransport` with three data-plane modes.** One WG packet
per `send`/`recv`. `Direct` = real UDP datagram to the entry gateway. `QuicBridge` =
length-framed over a reliable QUIC bi-stream to a bridge. The three supported modes
are one-hop, two-hop, and QUIC-tunnelling two-hop (QUIC only fronts the two-hop entry
leg — the bridge is 1:1 with a gateway).

**D6 — QUIC bridge reimplemented inline, mirroring `nym-vpn-client`.** Do **not**
depend on the `nym_bridges` crate (protocol `version = "0"`, unstable) — `nym-vpn-client`
itself declares `quinn` directly and reimplements the client. Byte-match three
invariants: ALPN `hq-29`; `IdentityBasedVerifier` (ed25519-only, SNI/CN ∈ alt-names,
SPKI == pinned `id_pubkey`); 2-byte big-endian length framing per WG packet over one
`open_bi()` stream. Client sets `keep_alive_interval` + `max_idle_timeout` + BBR for
the long-lived session. Bridge params (`{addresses, host(SNI), id_pubkey}`) come from
the gateway directory. Canonical reference:
`nym-vpn-core/crates/nym-vpn-lib/src/tunnel_state_machine/tunnel/transports/`.

**D7 — Registration is Direct-only.** LP registration (TCP to the gateway LP port)
runs first and is not bridgeable over QUIC yet (the bridge is a UDP relay to the WG
port). `GatewayTransport::QuicBridge` affects the data plane only. LP-over-QUIC is a
future extension the `LpTransportChannel` trait already permits.

**D8 — DNS resolves in-tunnel by default**, configurable (bind a resolver to a
`smol-core` UDP socket, per `smolmix`'s `mixdns`), so name resolution isn't leaked to
the host.

**D9 — MTU is configurable and dynamically changeable while the tunnel is up.**
Defaults from the reference: overhead 80 B/hop; desktop entry 1420 / exit 1340; mobile
entry 1360 / exit 1280. A config channel re-sizes the smoltcp interface + re-derives
per-hop MTUs at runtime.

**D10 — New third-party deps are crate-local.** `boringtun`, `quinn`, `quinn-proto`
declared in `sdk/rust/smol-dvpn/Cargo.toml`, not the workspace table; deps already in
the workspace table (`smoltcp`, `tokio`, `nym-*`) stay `workspace = true`.

## Risks / Trade-offs

- **Two-hop nesting correctness** → De-risk with a conformance spike (Task 1): register
  two hops and confirm one hand-framed packet round-trips vs. the reference. Mechanism
  is confirmed; parity is the only unknown.
- **QUIC bridge protocol drift** (`nym_bridges` is `version = "0"`) → Reimplement inline
  behind a small module + conformance test (Task 2); track the reference at a pinned
  commit; note `id_pubkey` is the base64 **public** key.
- **`smolmix` is published (`1.21.3`)** → Refactoring onto `smol-core` must preserve its
  public API; add regression coverage.
- **MTU mismatch / black-holing** → Default to reference values, subtract overhead
  correctly per hop, keep MTU configurable and runtime-adjustable.
- **boringtun timers** → Drive `Tunn::update_timers` from a dedicated cancellable task,
  routing keepalive/handshake/rekey output through the active transport.
- **Long-lived bandwidth exhaustion** → Background top-up task spends stored tickets via
  the gateway `metadata` endpoint before bandwidth runs out.

## Migration Plan

Additive: three new crates + one internal `smolmix` refactor. No runtime migration for
existing users. Rollout order mirrors the task phases: `smol-core` (with `smolmix`
regression parity) → `nym-sdk-session` → `nym-smol-dvpn` datapath → QUIC bridge →
example CLIs. Each crate lands behind its own tests; the two spikes gate the datapath
and bridge work. Rollback = do not add the new crates to the workspace `members`.

## Open Questions

Both resolved during exploration:

- **Single-hop vs two-hop surface (resolved).** The SDK expresses the mode by which
  gateways the caller names: **single-hop `gateway = identity_key | two_letter_country_code
  | random`**; **two-hop `entry = …` and `exit = …`**. Single-hop uses one WireGuard
  `Tunn` with no inner encapsulation.
  Implementation note: `nym-registration-client`'s `BuilderConfig` currently mandates
  **both** `entry_node` and `exit_node` (both `NymNodeWithKeys`), and `nym-vpn-client`
  only ever runs WireGuard two-hop — so single-hop is a genuinely new path.
  `nym-smol-dvpn` performs a **single-gateway registration** for single-hop (via the LP
  single-gateway `register_dvpn` path) rather than forcing `entry == exit` through the
  two-hop config; the exact registration entry point is nailed down in Task 3.7.
- **`smol-core` API boundary (resolved).** `smolmix`'s public surface **must remain
  unchanged**; `smol-core` is carved out beneath it and `smolmix` is refactored to
  consume it. Enforced by the `smol-core-stack` spec ("smolmix public API unchanged")
  and Task 2.5 regression coverage.

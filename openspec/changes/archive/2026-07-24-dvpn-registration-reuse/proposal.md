# Proposal: dvpn-registration-reuse

## Why

Every `nym-sdk-session` registration call generates a fresh WireGuard keypair
(`session.rs:606`, `session.rs:623`) and registers a brand-new peer on the
gateway, spending one zk-nym ticket per hop — even against a gateway the
client registered with moments ago. Nothing persists the keypair, the
assigned tunnel IPs, or the fact that a registration exists, so the previous
peer's remaining bandwidth allowance (each ticket grants ~477 MiB) is
abandoned on the gateway, unreachable forever. Observed on mainnet
(2026-07-23): two consecutive `zcash-sync` runs against the *same* entry and
exit gateways spent 4 tickets (~1.9 GiB of purchased allowance) to move
~500 MB — every repeated short-lived connection pays full price and strands
the change. A client that reconnects to a known gateway should reuse its
existing peer and spend nothing.

## What Changes

- Persist per-gateway registration state in the session's data directory:
  client WireGuard keypair, the gateway-returned `WireguardConfiguration`
  (already `Serialize`/`Deserialize`), gateway identity, role
  (entry/exit/single), and issuance metadata (registered-at, network name).
- On `register_single_hop` / `register_two_hop` / `register_two_hop_quic`
  against a gateway with cached state: return the cached registration
  WITHOUT contacting the gateway or spending a ticket. Fresh registration
  (and one-ticket spend) happens only when no usable cached entry exists.
- Cached registrations are validated by use, not by trust: the datapath
  gains an explicit "session established" signal (building on the
  handshake-progress instrumentation already in `smol-dvpn`'s engine) so a
  caller can bound tunnel bring-up; on failure it invalidates the cached
  entry via a new session API and re-registers fresh (spending a ticket) —
  the same funds-safety story as today, paid only when actually needed.
- New session APIs: cache consultation is built into the existing
  `register_*` calls (no caller change for the happy path), plus
  `invalidate_registration(gateway, role)` for the fallback path, and a
  `SessionConfig` opt-out for callers that require a fresh peer.
- `smol-dvpn`: `Tunnel` exposes an awaitable established-within-bound signal
  (per hop) so examples/apps can implement the reuse→validate→fallback loop
  without parsing logs.
- Examples (`zcash-sync`, `two-hop-ip`, `two-hop-quic`, `smol-dvpn-grpc`)
  adopt the loop: reuse when cached, fall back to fresh registration when
  the tunnel does not establish in time. Repeat runs against the same
  gateways spend zero tickets until the allowance actually depletes (at
  which point the existing metadata top-up flow spends from stored books).
- No `common/` production-code changes; everything lands in
  `sdk/rust/nym-sdk-session` and `sdk/rust/smol-dvpn`.

## Capabilities

### New Capabilities

_None — this extends existing capabilities._

### Modified Capabilities

- `dvpn-session`: gateway registration gains persistence and reuse —
  registrations are cached per (network, gateway, role); a cached
  registration is returned without a gateway exchange or ticket spend; a
  new invalidation API supports validate-by-use fallback; opt-out available
  for callers requiring a fresh peer.
- `dvpn-tunnel`: the tunnel exposes an awaitable per-hop
  session-established signal with a caller-chosen bound, making cached-
  registration validation (and dead-tunnel detection generally) a first-
  class API instead of a log-parsing exercise.

## Impact

- **Code**: `sdk/rust/nym-sdk-session` (registration cache module, session
  API additions, `SessionConfig` knob); `sdk/rust/smol-dvpn` (established
  signal on `Tunnel`, wired from the engine's existing progress markers);
  example CLIs updated to the reuse→validate→fallback pattern.
- **Data**: a new registration-cache file next to `creds.db` in the
  session data directory, containing WireGuard **private keys** — same
  sensitivity class as the credential store beside it (unencrypted secrets,
  file-permission hardening, documented).
- **Economics**: repeat connections to known gateways drop from
  one-ticket-per-hop-per-run to zero until allowance depletion; combined
  with the existing in-tunnel top-up, tickets are spent proportional to
  bandwidth actually used.
- **Compatibility**: no gateway/protocol changes — reuse rides on WireGuard
  peers the gateways already keep installed; existing callers keep working
  (reuse is the new default, opt-out provided). Concurrent processes
  sharing one data directory remain single-writer, as with `creds.db`.
- **Not affected**: `common/` crates, gateway/nym-node code, the QUIC
  bridge protocol (bridge parameters stay directory-sourced, not cached).

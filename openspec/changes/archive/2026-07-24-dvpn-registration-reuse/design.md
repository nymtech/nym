# Design: dvpn-registration-reuse

## Context

Confirmed on mainnet and in code: `register_two_hop_inner`
(`sdk/rust/nym-sdk-session/src/session.rs`) creates fresh
`x25519::KeyPair`s per call (lines 606/623) and `register_dvpn` spends one
ticket per hop, unconditionally. `Registration`/`HopConfig` (keys, assigned
IPs, gateway identity) live only in memory. Gateways keep the WireGuard peer
installed after the client disconnects, with the remaining allowance keyed
to the client's WG public key — so the *protocol* already supports
reconnection; only the client forgets.

Facts the design builds on:

- `WireguardConfiguration` (`common/registration/src/lib.rs:49`) is already
  `Serialize + Deserialize` (public_key bs58, psk, endpoint, private
  IPv4/IPv6). The client private key is an `x25519::PrivateKey`
  (32 bytes, bs58-encodable via nym-crypto helpers).
- The `smol-dvpn` engine already tracks per-hop handshake establishment
  (the one-shot info markers added for datapath observability, driven off
  boringtun's `stats().0`) — a machine-readable signal is a small extension.
- Sessions already own a per-instance data directory (`data_path`) holding
  secrets of the same sensitivity (`creds.db` ticketbooks).
- The in-tunnel top-up flow (`smol-dvpn` `topup.rs`, builder `.topup`)
  already exists for extending a live peer's allowance from stored books.
- The bring-up examples poll `ipinfo_via_tunnel` up to 10×3s today — the
  validate-by-use loop replaces blind polling with an explicit bound.

## Goals / Non-Goals

**Goals:**

- Reconnecting to a gateway the session has registered with before spends
  **zero tickets** while the cached registration still works.
- Cached state is validated by use with bounded bring-up, and invalidation +
  fresh registration is a first-class, funds-safe fallback.
- Registration state survives process restarts (same data directory), keyed
  per network + gateway + role, so mixed sandbox/mainnet and entry/exit
  usage never cross-contaminate.
- No behavior change for first-time registrations; opt-out for callers that
  need a guaranteed-fresh peer.

**Non-Goals:**

- Recovering allowance already stranded by past runs (previous keys were
  never saved; that value is gone).
- Client-side knowledge of gateway peer lifetime/GC policy — we never trust
  a cache entry's freshness; we try it under a bound and fall back.
- Multi-process sharing of one cache (same single-writer stance as
  `creds.db`).
- Caching QUIC bridge parameters (directory-sourced, can rotate) or LP
  key material (the LP handshake is per-connection by design).
- Encrypting the cache at rest beyond file permissions (parity with the
  adjacent credential store; revisit both together if requirements change).

## Decisions

### D1. Cache lives in `nym-sdk-session`, consulted inside `register_*`

The session owns registration, so it owns the cache. `register_single_hop`
/ `register_two_hop` / `register_two_hop_quic` first consult the cache for
each required (gateway identity, role) after gateway selection; on a full
hit they return a `Registration` assembled from cached state without any LP
exchange. Partial hits (e.g. entry cached, exit not) register only the
missing hop — spending only that hop's ticket.

- Alternative (rejected): a separate `try_cached_registration()` API the
  caller must remember to use — silently keeps the expensive path the
  default and splits the flow across every caller.
- Two-hop QUIC: the WG registration is cached identically; bridge params
  are re-resolved from the directory each time (Non-Goal above).

### D2. Storage: one JSON file per session data dir, atomic writes

`registrations.json` next to `creds.db`: a versioned
`{ version, entries: [...] }` document, each entry
`{ network, gateway_identity (bs58), role, client_private_key (bs58),
wg_config (WireguardConfiguration), registered_at }`. Written via
temp-file + rename (atomic), created with 0600 permissions on unix.

- Why JSON over sqlite: tiny (a handful of entries), no query needs, no
  migration machinery, trivially inspectable when debugging gateway issues.
- Keyed by network name so one data dir can serve sandbox and mainnet runs
  without cross-network reuse (the example dirs are already per-network;
  the key makes the invariant hold even when they're not).
- The whole file is the unit of write; `Zeroize` on the in-memory private
  key mirrors the session's existing mnemonic handling.

### D3. Validate by use: bounded establishment, then invalidate + retry

The cache never answers "is this registration still good?" — the tunnel
does. `smol-dvpn`'s `Tunnel` gains
`await_established(timeout) -> Result<(), NotEstablished>` (covering all
hops: entry, and exit when two-hop), implemented as a `tokio::sync::watch`
the datapath task updates from the engine's existing establishment markers.
The reuse loop (in the caller, demonstrated by the examples):

1. `register_*` → cached `Registration` (no spend).
2. Build tunnel → `await_established(ESTABLISH_BOUND)` (reference: 15s —
   observed healthy two-hop establishment is ~120ms).
3. On timeout: tear down, `session.invalidate_registration(gateway, role)`
   for the failed hop(s), `register_*` again (fresh spend), rebuild.

- Why caller-driven rather than auto-fallback inside the session: the
  session doesn't own the datapath (layering: provisioning facade vs.
  `smol-dvpn`), and callers may want different bounds/policies. The loop is
  ~10 lines and lives in `examples/common` for reuse.
- `await_established` is generally useful beyond caching (replaces the
  examples' blind 10×3s ipinfo polling for failure detection).

### D4. Which hop failed: per-hop establishment status

`await_established` failure distinguishes hops
(`NotEstablished { entry: bool, exit: Option<bool> }`) so the fallback
invalidates only the dead hop's cache entry — an expired entry peer doesn't
force re-buying the still-valid exit registration (and vice versa). The
watch value carries both flags; the engine already tracks them separately.

### D5. Reuse is default-on; `SessionConfig::reuse_registrations: bool`

Default `true`. Opt-out exists for callers that require an unlinkable fresh
peer per connection (privacy posture) or are debugging gateway state.
Invalidation API: `invalidate_registration(&self, gateway: &ed25519::PublicKey,
role: WgRole)` removes the entry and persists; missing entries are a no-op.

- Privacy note (documented on the config field): reusing a WG key links a
  client's sessions to the same peer identity at the gateway across
  connections — that's inherent to reusing the allowance. Callers wanting
  per-connection unlinkability opt out and pay per connection.

### D6. Cache write happens on successful registration, inside the session

`register_hop` / `register_two_hop_inner` persist each hop's entry
immediately after `register_dvpn` returns (the spend already happened;
persisting is what prevents the *next* spend). A failed persist logs a
warning and does not fail the registration — worst case is today's
behavior (one extra future spend), never a lost registration.

### D7. Expiry hygiene without trusting it

Entries carry `registered_at`; entries older than a conservative
`MAX_CACHE_AGE` (reference: 30 days) are treated as absent (and pruned on
save) purely to bound file growth and skip hopeless attempts. This is an
optimization, not a validity oracle — D3 remains the actual validation.

### D8. Tests: same fixture strategy as the signer-failure suite

- Session-level (nym-sdk-session): cache round-trip (persist → new session
  instance → cache hit returns equivalent `Registration` without invoking
  the provider — assert **zero** `get_ecash_ticket` calls via a counting
  mock `BandwidthTicketProvider`); invalidation removes exactly the keyed
  entry; network-name isolation; opt-out bypasses the cache; corrupt/absent
  file → treated as empty (fresh registration path), never a crash.
  Registration itself is faked at the seam the session already exposes for
  tests (the gateway exchange is not unit-testable without a gateway; the
  cache logic is factored so persistence/lookup are testable pure of LP).
- Datapath-level (smol-dvpn): `await_established` resolves once the engine
  marks establishment (loopback UDP pair like the existing transport
  tests), times out with per-hop flags when handshakes never complete.
- Live validation (manual, documented): two consecutive mainnet runs
  against the same gateways — second run logs the cache hit and the
  ticketbook `used_tickets` count is unchanged.

## Risks / Trade-offs

- [Gateway removed the peer but WG handshake to a *different* live peer
  cannot be distinguished from network loss within the bound] → Accepted:
  either way the entry is invalidated and a fresh registration follows;
  cost equals today's per-run spend, paid only on actual failure.
- [Private keys at rest in a JSON file] → Same trust boundary and directory
  as `creds.db` ticketbook secrets; 0600 permissions; documented. Encrypting
  one without the other buys nothing.
- [Reused assigned IP while gateway reassigned it to someone else] →
  Gateway-side cryptokey routing ties the IP to the WG public key; a
  conflicting reassignment manifests as non-establishment or no data →
  D3 fallback covers it.
- [Two processes sharing a data dir reuse the same WG key concurrently] →
  Sessions from one data dir are single-client by existing convention
  (credential store); documented. Last-writer-wins on the JSON is not
  data-loss (worst case: an extra future spend).
- [Session linkability across connections at the gateway] → Inherent to
  allowance reuse; surfaced as a documented opt-out (D5), not hidden.
- [Examples' fallback loop adds a bounded delay (up to ESTABLISH_BOUND)
  when a cached registration is dead] → 15s once, versus spending a ticket
  every run; acceptable and tunable.

## Migration Plan

Additive. Existing data dirs simply gain `registrations.json` on the next
successful registration; absence of the file = today's behavior. No
`SessionConfig` breakage (new field defaults on; struct is constructed with
struct-update or builder patterns in-tree — verify call sites during
implementation). Rollback = revert; stale cache files are ignored by old
code and can be deleted freely.

## Resolved Questions (stakeholder decisions, 2026-07-24)

- **`await_established` replaces the ipinfo retry loop entirely** — one
  bring-up discipline everywhere: every example gates on
  `await_established(ESTABLISH_BOUND)` after building the tunnel (cached
  or fresh), and ipinfo shrinks to a single display probe. The 10×3s
  blind-polling loops are removed.
- **Values fixed** (constants, not config, until a caller needs otherwise):
  - `ESTABLISH_BOUND = 15s`: healthy two-hop establishment is ~120ms
    observed on mainnet; WireGuard retransmits handshakes every ~5s, so
    15s allows ~3 attempts before declaring a hop dead — same order as
    the signer-fetch timeout, keeping "how long a dead component may
    stall us" uniform across the SDK.
  - `MAX_CACHE_AGE = 30 days`: pure file hygiene (D7) — real validity is
    always established by use (D3), so the age bound only prunes entries
    old enough to be near-certainly GC'd gateway-side, without discarding
    plausibly live peers.
- **Cache hits are logged at info** — `reusing cached registration for
  <gateway> (<role>)` — so the zero-spend behavior is obvious in exactly
  the logs where the per-run ticket drain was first noticed. Symmetrically,
  a fallback logs at warn (`cached registration for <gateway> (<role>)
  failed to establish; re-registering`).

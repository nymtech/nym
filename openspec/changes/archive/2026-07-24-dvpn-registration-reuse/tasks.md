# Tasks: dvpn-registration-reuse

## 1. Registration cache (nym-sdk-session)

- [x] 1.1 Create a `registration_cache` module: versioned
      `registrations.json` document model — entries keyed by
      (network name, gateway identity bs58, role) holding the client WG
      private key (bs58, zeroized in memory), the
      `WireguardConfiguration`, and `registered_at`
- [x] 1.2 Implement load (absent/corrupt file → empty, log warn at most)
      and atomic save (temp file + rename, 0600 permissions on unix),
      with `MAX_CACHE_AGE` pruning on save and lookup
- [x] 1.3 Add `SessionConfig::reuse_registrations: bool` (default `true`;
      doc comment covering the linkability trade-off of reusing a WG peer
      identity) — check all in-tree `SessionConfig` construction sites
- [x] 1.4 Consult the cache in `register_single_inner` /
      `register_two_hop_inner` after gateway selection: full hit returns a
      cache-assembled `Registration` (no LP exchange, no spend); partial
      two-hop hit registers only the missing hop; log a cache hit at info
      (`reusing cached registration for <gateway> (<role>)`)
- [x] 1.5 Persist each hop's entry immediately after its successful
      `register_dvpn` (persist failure = warn, never fails registration)
- [x] 1.6 Add `Session::invalidate_registration(gateway, role)` — removes
      the entry and persists; absent entry is a no-op

## 2. Awaitable establishment (smol-dvpn)

- [x] 2.1 Publish per-hop establishment from the engine's existing
      progress tracking through a `tokio::sync::watch` channel owned by
      the datapath task (values: entry established, exit established)
- [x] 2.2 Add `Tunnel::await_established(timeout) -> Result<(),
      NotEstablished>` where `NotEstablished` carries per-hop status
      (`entry: bool`, `exit: Option<bool>`); works for single-hop,
      two-hop, and QUIC-bridged tunnels

## 3. Reuse→validate→fallback loop (examples)

- [x] 3.1 Add the loop to `examples/common`: register (cache-served when
      possible) → build tunnel → `await_established(ESTABLISH_BOUND = 15s)`
      → on failure log at warn (`cached registration for <gateway> (<role>)
      failed to establish; re-registering`), tear down, invalidate the
      failed hop(s), re-register, rebuild once
- [x] 3.2 Adopt it in `zcash-sync`, `two-hop-ip`, `two-hop-quic`
      (and `smol-dvpn-grpc` inline), replacing the blind 10×3s
      ipinfo warmup retry as the establishment gate (keep one ipinfo probe
      for the IP display)

## 4. Tests

- [x] 4.1 Cache unit tests: round-trip through a fresh session/cache
      instance; (network, gateway, role) keying and network isolation;
      invalidation removes exactly the keyed entry; corrupt/absent file →
      empty; pruning by age; file permissions (unix)
- [x] 4.2 Session-level reuse test with a counting mock
      `BandwidthTicketProvider`: cache hit produces a `Registration`
      with **zero** `get_ecash_ticket` calls; opt-out bypasses the cache;
      partial two-hop hit spends exactly one
      (factor the cache lookup/assembly so it is testable without an LP
      exchange; if the LP seam resists mocking, cover assembly + spend
      accounting at the cache-module boundary and note the gap)
      — implemented via the escape hatch: `cached_hop` was refactored to
      take (identity, GatewayInfo) so the reuse seam is testable offline;
      4 session-level tests cover zero-spend cache hits, opt-out
      no-read-no-write, invalidation, and restart survival. The full
      partial-two-hop LP path needs a live gateway (nym-api topology +
      LP exchange resist offline mocking) and is exercised by the manual
      mainnet validation (5.3) instead.
- [x] 4.3 `await_established` tests in smol-dvpn: resolves on
      establishment (loopback UDP pair, as in existing transport tests);
      times out with correct per-hop flags when handshakes never complete
      (unresponsive peer), including exit-only failure attribution
- [x] 4.4 Run `cargo test -p nym-sdk-session -p nym-smol-dvpn` and
      `cargo clippy --tests` for both crates — green, no warnings

## 5. Verification & documentation

- [x] 5.1 Confirm `git diff --stat` touches only `sdk/rust/nym-sdk-session`,
      `sdk/rust/smol-dvpn`, and openspec files (no `common/`)
- [x] 5.2 Document the cache in both crates' rustdoc + smol-dvpn README:
      what is persisted and where, the zero-spend reuse behavior, the
      validate-by-use fallback, the opt-out and its privacy rationale
- [x] 5.3 Manual mainnet validation (documented): two consecutive runs
      against the same gateways — second run logs the cache hits,
      `used_tickets` unchanged in `creds.db`, tunnel establishes and
      passes traffic; then a forced-stale check (delete the gateway pair
      from the cache of a *different* gateway… or invalidate manually) to
      exercise the fallback path once — VALIDATED on mainnet 2026-07-24
      by the crate owner (run also doubled as smoldvpn-rename task 4.1:
      post-rename live run with cached registrations reused)

# Tasks: Add IPR Connect Retry

## 1. Discovery returns a candidate list

- [x] 1.1 Change `discover_ipr` (`wasm/smolmix/src/ipr.rs`) to return an ordered `Vec<(Recipient, Version)>` of all eligible exits, performance-weighted ordering, instead of a single `choose_weighted` draw
- [x] 1.2 Keep the existing filters unchanged (exit-IPR role, version at or above `v9::MIN_RELEASE_VERSION`, parseable `Recipient`)
- [x] 1.3 Return an explicit error when the candidate list is empty (no eligible exits), distinct from a handshake failure

## 2. Per-version handshake (fallback kept)

- [x] 2.1 Run `open_and_connect` with its v10-then-v9 fallback per exit; do NOT skip it (nodes advertise v10 but run v9). Corrected after testing.
- [x] 2.2 Bound each version by `ipr_attempt_timeout` via `open_and_connect`'s internal per-version timeouts; do NOT wrap the whole handshake in a single outer timeout (it strangles the v9 fallback)

## 3. Rotation loop in tunnel setup

- [x] 3.1 In `WasmTunnel::new`, gate the rotation loop on the auto-discovery path (`opts.ipr_address` is `None`)
- [x] 3.2 Iterate the candidate list, handshake each, rotate on handshake error, bounded by `ipr_max_attempts` and list length; on exhaustion fail fast (no slow retry)
- [x] 3.3 Leave the pinned-IPR path (`opts.ipr_address` is `Some`) on the existing single-attempt behaviour with its cross-version fallback

## 4. Tuning options

- [x] 4.1 Add `ipr_attempt_timeout` (default 6 s, per-version) and `ipr_max_attempts` (default 5) to `TuningOpts` with defaults and builder setters
- [x] 4.2 Expose as JS `SetupOpts.iprAttemptTimeoutMs` / `iprMaxAttempts`; both take effect only on the auto-discovery path
- [x] 4.3 Add a live `IPR attempt timeout` input to the internal-dev tool

## 5. Observability

- [x] 5.1 Log each failed attempt with the exit address, elapsed time, and failure reason
- [x] 5.2 Log a v10-awaited-but-vN-received skip in `connect_v10`, so directory-vs-running version skew is visible instead of a silent stall
- [ ] 5.3 Log the final outcome (which exit connected, or total failure after N attempts)

## 6. v10 MTU-tolerance (compat shim)

- [x] 6.1 In `connect_v10`, when a v10-tagged response fails to parse as v10, parse it as v9 (v10 == v9 + trailing mtu) and synthesise `CLIENT_MTU_FALLBACK` (1420), gated on the request-id match, rather than dropping it and eating the v10 timeout. smolmix-only; no shared-crate change.

## 6. Tests

- [ ] 6.1 First candidate times out, second succeeds: tunnel establishes against the second exit
- [ ] 6.2 Empty candidate list: establishment returns the no-eligible-exits error, not a timeout
- [ ] 6.3 Pinned IPR: no rotation, single attempt, existing behaviour preserved
- [ ] 6.4 `ipr_max_attempts` caps rotation even when more candidates remain
- [ ] 6.5 All candidates fail: establishment fails fast (bounded by attempts × timeout), not a 60 s stall

## 7. Out of scope

- Entry-gateway rotation. Only the exit IPR rotates; the sticky entry-gateway registration is unchanged.
- Re-fetching the directory between attempts. The candidate list is fetched once and reused.
- Any IPR-side change. This is client establishment logic only.

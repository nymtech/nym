# Add IPR Connect Retry

## Why

The WASM tunnel (`nym-smolmix-wasm`, the mix-fetch product) picks its exit IP packet router once and commits to it. `discover_ipr` fetches every eligible exit, then collapses the list to a single performance-weighted pick and discards the rest. `WasmTunnel::new` then runs one handshake against that pick with a 60 second `connect_timeout`. If that one IPR is dead, overloaded, or black-holing, the caller waits up to a minute and then the whole tunnel setup fails. There is no attempt at a different exit.

The only retry that exists today is the v10-then-v9 protocol fallback inside a single handshake (`open_and_connect`), which retries a different protocol version against the *same* IPR. Nothing re-picks a different IPR.

This change makes auto-discovery establishment fail fast per exit and rotate to a different exit, so a bad IPR costs a few seconds instead of a minute, and connection succeeds as long as any eligible exit is healthy.

## What Changes

- `discover_ipr` returns an ordered candidate list instead of one pick. It keeps the existing filters (exit-IPR role, version at or above `v9::MIN_RELEASE_VERSION`, parseable `Recipient`) and keeps performance as the ordering weight, but produces a performance-weighted ordering of every eligible exit rather than a single weighted draw. The directory is fetched once; rotation reuses the retained list and does not re-fetch.
- `WasmTunnel::new`, on the auto-discovery path only (`TunnelOpts.ipr_address` is `None`), attempts the IPR handshake against candidates in order with a short per-exit timeout (`ipr_attempt_timeout`, a new `TuningOpts` field, default 6 seconds), rotating to the next candidate on timeout or handshake error. Rotation is bounded by a maximum attempt count (`ipr_max_attempts`, a new `TuningOpts` field, default 5) and by the candidate list length.
- Each attempt runs the v10-then-v9 cross-version fallback. An earlier draft skipped it on the assumption that the directory version is authoritative, but live testing showed that many nodes advertise v10 in the directory while their running process answers only v9, so the v10 probe gets no reply and the connect must fall through to v9 to succeed. The fallback is bounded per version, not by a single outer timeout across the whole handshake; a single outer bound strangles the v9 fallback and abandons exits that are actually reachable.
- The budget is a per-version timeout (default 6 seconds), so one exit may spend up to two of them: a v10 probe, then a v9 connect. A single mixnet handshake round trip is second-scale, and a healthy connect is sub-second (measured against a pinned exit), so 6 seconds is a generous per-version probe. It is a `TuningOpts` knob, exposed to JS as `iprAttemptTimeoutMs`, because the right value tracks live mixnet latency, which drifts.
- The pinned-IPR path (`TunnelOpts.ipr_address` is `Some`) is unchanged. With a single fixed exit there is nothing to rotate to, so fast-fail rotation is auto-discovery only. This matches the operator expectation that pinning an IPR is a deliberate override.
- Each failed attempt logs the exit address, the elapsed time, and the failure reason, so a black-holing exit is visible in the console rather than hidden inside one long stall.

## Capabilities

### New Capabilities

<!-- none: this change modifies the smol-core-stack capability -->

### Modified Capabilities

- `smol-core-stack`: adds fast-fail IPR establishment with exit rotation to the WASM tunnel setup path, on top of the existing single-pick discovery and single-handshake establishment.

## Impact

- `wasm/smolmix/src/ipr.rs`: `discover_ipr` returns `Vec<(Recipient, Version)>` (ordered) instead of one pick; the weighted-single-draw becomes a weighted ordering. `open_and_connect` is called per exit with its v10-then-v9 fallback intact; the outer per-exit timeout is removed so the fallback is not strangled.
- `wasm/smolmix/src/tunnel.rs`: the IPR resolution block (currently lines around 287-307) and `ipr_handshake` become a rotation loop over the candidate list on the auto path; `TuningOpts` gains `ipr_attempt_timeout` and `ipr_max_attempts` fields, defaults, and builder setters.
- No wire-protocol change: this is client-side establishment logic only. IPRs are unchanged.
- The `TUNNEL` `OnceLock` guard in `lib.rs` is unaffected: rotation happens inside `WasmTunnel::new`, before the tunnel is stored, so a retry never re-enters `setupMixTunnel`.
- Entry-gateway registration is untouched. Rotation changes only the exit IPR, not the sticky entry gateway the base client registered with.

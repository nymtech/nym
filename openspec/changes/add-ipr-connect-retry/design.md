# Design: Add IPR Connect Retry

## Context

`WasmTunnel::new` establishment, auto-discovery path. Recon references are to `wasm/smolmix/src/ipr.rs` and `wasm/smolmix/src/tunnel.rs` as of this change.

Current establishment (auto path):
1. `discover_ipr` fetches the whole directory, filters to eligible exits, and returns one performance-weighted pick, discarding the rest.
2. `ipr_handshake` runs `open_and_connect` against that pick: an LP Open frame, then a Connect at v10 (bounded 15 s by `IPR_V10_ATTEMPT_TIMEOUT`), falling back to v9 on timeout. The whole handshake is bounded by `connect_timeout` (default 60 s).

The failure mode: one bad exit costs up to 60 s and then fails the tunnel, with no other exit tried.

## Goals

- Fail per exit in a few seconds, not a minute.
- Rotate to a different exit on failure, so establishment succeeds whenever any eligible exit is healthy.
- Do not regress the pinned-IPR path or the slow-but-live exit case.

## Decisions

### Weighted ordering, not repeated weighted draws

`discover_ipr` produces one performance-weighted ordering of all eligible exits and returns the whole list. Rotation walks it in order. This keeps the existing bias toward higher-performance exits (the best exit is most likely first) while giving deterministic, non-repeating rotation. The alternative, re-drawing `choose_weighted` per attempt, can draw the same dead exit twice and needs de-duplication. Ordering once is simpler and strictly better here.

### Attempts run the cross-version fallback (revised after testing)

An earlier draft skipped the v10-then-v9 fallback on the auto path, reasoning that the directory version is authoritative so each attempt could connect at the advertised version directly. Live testing refuted this. Against three different entry gateways, auto-discovery rotated through exits that each logged `v10 connect timed out; retrying v9`: the nodes advertise v10 in the directory but their running process answers only v9 (the directory version leads the running process, the exact case the fallback exists for). A pinned exit connected sub-second because it used the full budget that lets v9 run.

The original design also wrapped the whole handshake in a single per-exit `timeout`. That is the actual bug: v10 ate the entire budget, and when the connect fell through to v9, the outer timeout had already fired, so v9 never sent. The fix is to drop the outer wrapper and let `open_and_connect`'s internal per-version timeouts bound the attempt. `attempt_timeout` becomes the per-version budget; a single exit may spend up to two (v10 probe, then v9 connect) before rotating. Worst case per exit is two budgets, but a v9-reachable exit connects within roughly one budget plus a fast v9 round trip.

### No slow safety-net

An earlier draft kept a final slow attempt (full `connect_timeout`, v10-then-v9 fallback) after rotation exhausted. It is cut: it reintroduces the 60-second stall this change exists to remove, and if several eligible exits all fail a 6-second probe the network is almost certainly at fault, not the budget. Rotation is uniform: N attempts at timeout T, no special last attempt. If every candidate fails, establishment fails fast with a clear per-attempt log, which is the better outcome than one more minute of waiting.

### Budgets

- `ipr_attempt_timeout` default 6 s: long enough to clear a normal second-scale mixnet handshake round trip, short enough to rotate. 3 s (first sketched) risks abandoning healthy exits on a cold connection.
- `ipr_max_attempts` default 5: caps total rotation cost. Five exits at up to two 6 s version probes each is the rotation ceiling; a v9-reachable exit connects in about one probe plus a fast v9 round trip.
- Both are `TuningOpts` fields, overridable from JS, and only take effect on the auto path. The right timeout tracks live mixnet latency, which drifts, so it stays a knob rather than a constant.

## Interactions and risks

- OnceLock guard: rotation is internal to `WasmTunnel::new`, before the tunnel is stored in `TUNNEL`. A retry never re-enters `setupMixTunnel`, so the double-init guard is not tripped.
- Entry gateway: rotation changes only the exit IPR. The base client's entry-gateway registration is sticky per client id and is not re-rolled. A tunnel that cannot register with any entry gateway fails before IPR selection and is out of scope here.
- SURB cost: each failed attempt sends an Open frame with reply SURBs that are never consumed. Five attempts is a small, bounded SURB spend. Worth a note but not a blocker.
- Directory fetch cost: fetched once, reused across attempts. No extra directory load from rotation.

## Non-goals

- Entry-gateway rotation.
- Concurrent (hedged) attempts against multiple exits at once. Sequential rotation is simpler and the per-attempt budget already bounds latency; hedging is a possible later optimisation.
- Any change to the IPR or the wire protocol.

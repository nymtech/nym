# Add Entry-Gateway Rotation

## Why

`add-ipr-connect-retry` rotates the exit IP packet router when a handshake fails, and it works: live testing shows the tunnel fast-failing one exit and trying a different one. But it exposed a failure mode exit rotation cannot fix. When every exit attempt times out in the same way, the fault is usually not the exits, it is common-mode: the entry gateway the base client registered with, or the send path through it. Rotating the exit is futile if packets cannot get through the entry gateway in the first place.

This change adds an escalation tier: when exit rotation fails against a bounded number of distinct exits, treat the failure as likely common-mode and re-register with a different entry gateway, then retry exit selection on the new entry.

Note on motivation: the incident that first suggested this (all exits timing out against entry gateway `koutakou.cloud`) was later traced to a different cause, a per-exit timeout that strangled the v10-then-v9 fallback, fixed in `add-ipr-connect-retry`. The same failure appeared against three different entry gateways, and a pinned exit connected sub-second, so the entry gateways were not at fault. This change is therefore speculative resilience for a genuinely dead entry gateway, not a fix for that incident. See `design.md`.

## What Changes

- After a bounded number of distinct exits (default 3) all fail their handshake on the current entry gateway, establishment escalates: it re-registers the base client with a different entry gateway and retries exit discovery and rotation on the new entry.
- Entry rotation is re-registration, not a live repoint. Entry-gateway registration is sticky: it derives a per-client shared key with the gateway, so an established client cannot be repointed to a different gateway without registering afresh. The escalation therefore tears down and re-registers, which is heavier than exit rotation and is why it is a separate, later tier rather than the first response to a failure.
- The escalation is bounded (a small number of entry gateways, default 3) so a genuine network-wide outage terminates with a clear error rather than cycling entry gateways forever.
- Entry selection reuses the base client's existing performance-weighted gateway selection; a rotated entry excludes the one that just failed.

## Non-goals

- Rotating the entry gateway on a healthy, established tunnel. This is establishment-time recovery only.
- Preserving client identity across entry rotation beyond what re-registration already does.
- Distinguishing entry-gateway faults from a slow-but-live mixnet with certainty. The trigger is a heuristic (repeated common-mode exit failure), deliberately simple.

## Capabilities

### New Capabilities

<!-- none: this change modifies the smol-core-stack capability -->

### Modified Capabilities

- `smol-core-stack`: adds an entry-gateway rotation escalation to tunnel establishment, on top of the exit rotation added by `add-ipr-connect-retry`.

## Impact

- Builds on `add-ipr-connect-retry`. The exit-rotation loop in `WasmTunnel::new` gains an outer escalation: on exhaustion, re-run `start_nym_client` gateway registration against a different entry gateway and retry.
- `wasm/smolmix/src/tunnel.rs`: `start_nym_client` currently registers a gateway once; escalation needs a re-registration path that excludes the failed gateway. Registration is sticky per `client_id`, so re-registration implies a fresh gateway key exchange.
- Diagnostic-gated: this is worth building only if the diagnostic confirms the common-mode failures are the entry gateway rather than an over-aggressive per-exit timeout. The per-exit budget was found to be too short in first testing (5 s against a cold-mixnet handshake the code elsewhere budgets 15 s for), so the timeout must be ruled out first.
- New tuning: `ipr_max_entry_attempts` (default 3) bounds entry rotation.

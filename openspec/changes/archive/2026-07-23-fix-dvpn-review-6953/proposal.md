# Proposal: fix-dvpn-review-6953

## Why

PR #6953 (`feature/nym-sdk-dvpn`) received a CHANGES_REQUESTED review (jstuczyn: 33
inline comments + a review-body summary; mfahampshire: lgtm on `smol-*`). Every
substantive claim was verified against the code and holds up, including three
blocker-grade defects (top-up traffic bypassing the tunnel, a datapath-killing
`set_mtu` race, and a CPU-burning transport busy-loop). Separately, the branch was
rebased onto Simon Wicky's bandwidth-controller rework (#6898/#6937/#6952), which
makes `nym-sdk-session`'s current one-shot controller usage the deprecated
pattern — and provides the machinery that resolves the review's largest item
(self-sustaining bandwidth top-up) properly.

## What Changes

- **Upstream bandwidth crates** (lands as a SEPARATE PR against `develop` for
  focused review by Simon Wicky; this change depends on it):
  - `nym-bandwidth-fetcher`: additive `with_allowed_ticketbook_types(...)` on
    `NyxdCredentialFetcher` — refuses on-chain deposits for ticket types outside
    the allowed set (recovery of already-paid deposits still runs). No existing
    signature changes; default = current behavior (NymVPN unaffected).
  - `nym-bandwidth-controller`: additive `with_managed_ticket_types(...)` on
    `BandwidthController` scoping the proactive restock sweep. Default = current
    all-types sweep.
  - **Hard requirement**: a dVPN-only session must never deposit for mixnet
    ticket types (one ticketbook ≈ 37.5 GB; over-requesting costs implementers NYM).
- **Session architecture** (`nym-sdk-session`): migrate from one-shot controller
  calls to the running-controller pattern (spawn `run()`, operate via
  `BandwidthControllerRequestSender`), following client-core's
  `start_bandwidth_controller`, with an external-provider escape hatch preserving
  the single-writer invariant. Chain-side restock is **opt-in** via
  `with_automatic_topups(...)`; gateway-side top-up (spending already-stored
  tickets) is **on by default** for session-built tunnels. Dissolves the
  funds-safety review comments (`session.rs:174`, `:189`).
- **Datapath blockers** (`nym-smol-dvpn`): route the metadata/top-up client
  through the tunnel (privacy blocker); fix the `set_mtu` stack-swap race and
  concurrent-call desync; classify transport recv errors (fatal vs transient) to
  end the busy-loop; IP-literal connect short-circuit; prefer IPv4 bridge
  addresses for v4-only clients.
- **Bandwidth events**: new subscribable `BandwidthEvent` stream
  (`tunnel.bandwidth_events()`) decoupling bandwidth *monitoring* from automatic
  *top-up*, so implementers can notify users to purchase ticketbooks manually.
- **DNS hardening** (`smol-core`): validated per-query random transaction ids +
  source checks, recv-until-match loop, RCODE-aware errors, skip AAAA while the
  stack is v4-only.
- **Performance**: one focused datapath allocation pass (engine/transport/
  bridge/framing scratch buffers); single topology fetch per two-hop
  registration.
- **API polish + review-thread replies**: `#[must_use]` builders, typed WG keys
  via boringtun's dalek re-exports, redacted-field `Debug`, mobile MTU defaults by
  target OS, mnemonic zeroization + hashed client-id derivation, error-variant
  cleanup, doc corrections, and written replies on the threads where we push back
  (port 54001 semantics, top-up default rationale, topology endpoint question).

No breaking changes to published APIs: `nym-sdk-session`/`nym-smol-dvpn` are
unpublished (`publish = false` / new in this branch), and all upstream crate
changes are additive.

## Capabilities

### New Capabilities

- `bandwidth-type-scoping`: restricting ticketbook acquisition (fetcher deposits
  and controller restock sweeps) to an allowed set of ticket types, additively and
  default-off, in `nym-bandwidth-fetcher` / `nym-bandwidth-controller`.

### Modified Capabilities

- `dvpn-session`: provisioning moves to the running bandwidth controller
  (single-writer, sender-based spending); restock becomes opt-in policy with
  scoped ticket types; storage/cancellation funds-safety requirements; hashed
  client-id derivation and mnemonic zeroization.
- `dvpn-tunnel`: top-up traffic MUST travel in-tunnel; gateway-side top-up
  default-on for session-built tunnels; bandwidth event stream; runtime MTU
  change must not kill the datapath; transport failure handling requirements.
- `smol-core-stack`: IP-literal handling in `tcp_connect_host`; DNS response
  validation (id/source/RCODE) and v4-only AAAA suppression.

## Impact

- **Crates**: `common/bandwidth-fetcher`, `common/bandwidth-controller`
  (additive, separate PR); `sdk/rust/nym-sdk-session`, `sdk/rust/smol-dvpn`,
  `common/smol-core` (this PR).
- **External consumers**: NymVPN's integration of the bandwidth controller and
  fetcher is untouched (no signature changes, no new enum variants in
  `FetcherErrorKind`, defaults preserve behavior).
- **PR process**: unblocks re-review of #6953; several review threads get
  written replies (with rationale) rather than code changes.
- **Docs/specs**: `docs/design/sdk/smol-dvpn/design.md` §10 corrected (top-up
  behavior) and the bridges-publication TODO removed; spec deltas as listed above.

# Design: fix-coderabbit-findings-6953

## Context

Follow-up fixes to PR #6953 from CodeRabbit's automated pass. Each was verified
against the current rebased branch; this change addresses only the findings that are
genuinely actionable and in scope. The branch already sits on `develop` (with the
merged bandwidth-controller rework), and the datapath is validated end-to-end by the
live sandbox bring-up.

## Goals / Non-Goals

**Goals:**
- Close the registration lost-ticket cancellation window.
- Make the DNS resolver and QUIC bridge robust to truncated responses and stalled
  peers.
- Remove a misleading dead config knob and an unpinned-bridge gap.
- Make the README and design doc accurately describe the (scoped) guarantees.

**Non-Goals:**
- DNS-over-TCP retry on truncation (reject-and-error is sufficient here; the tunnel
  DNS is a single upstream and answers are small).
- Reworking the `deny.toml` AGPL policy (pre-existing, team-owned).
- Touching archived OpenSpec records or the `smolmix` crate.

## Decisions

### D1: Registration non-cancellable past ticket-spend
`register_single_hop` / `register_two_hop` / `register_two_hop_quic` currently wrap
their `_inner` (which spends a ticket via `register_dvpn` / `handshake_and_register_dvpn`)
in `tokio::select!` against the caller's token. Drop that race: check
`cancel.is_cancelled()` up front (and after gateway *selection*, which is safe to
abort), then run the registration exchange to completion without racing cancel. This
matches the accepted `ensure_ticketbooks` treatment — the deposit/spend path is not
abortable mid-flight. Gateway selection (before any spend) remains promptly
cancellable.

*Alternative considered:* a finer-grained "cancellable until first spend" guard
inside the LP client — rejected as a much larger change for a single low-value
ticket; the coarse "not cancellable once the exchange starts" boundary is adequate.

### D2: Reject truncated DNS responses
In `query_one`, after parsing the response and validating id/source, check the
header `truncated` flag; if set, return a distinct error (`SmolCoreError::DnsProto`
or a new `DnsTruncated`) instead of extracting whatever answers fit. This keeps a
partial/again-parseable-but-incomplete answer from being treated as authoritative.

### D3: Bound `open_bi()` in the QUIC bridge
Move `conn.open_bi()` inside the existing `tokio::select!`(cancel) + `timeout`
structure used for `transport_conn()`, so both the handshake and the first bi-stream
open honor `CONNECT_TIMEOUT` and the cancel token.

### D4: MTU config knob
`StackConfig::mtu`/`with_mtu` is not read by `Stack::new` (the caller-built
`ChannelDevice` fixes the MTU). Prefer **removing** `mtu` from `StackConfig` (and
`with_mtu`, `DEFAULT_MTU` usage there) so there's no dead lever; the tunnel already
passes the MTU to `ChannelDevice` directly. If any caller relies on
`StackConfig::mtu`, keep the field but have `Stack::new` build its device from it —
decide during implementation based on call sites.

### D5: Reject empty QUIC identity pin
In `dvpn.rs` bridge parsing, treat a blank `id_pubkey` like a blank `sni_host` /
empty address list: do not construct a `QuicBridge`, so `has_quic` never advertises
an unpinned bridge.

### D6: Test hygiene
Remove `std::process::exit(0)` from `live_bringup.rs`; `tokio::time::timeout` around
`tunnel.shutdown()` already bounds teardown. Keep the `--test-threads=1` doc note
(the two tests share one chain account).

### D7: Doc accuracy
- README: reword the kill-switch/leak line to state that only traffic through
  tunnel-provided sockets is protected; ordinary sockets and host DNS bypass it.
- Design doc: note that the "decoy" SNI is what a DPI observer sees, while the
  pinning verifier constrains the presented SNI to an accepted cert alt-name; and
  that top-up fires at a threshold leaving headroom so the in-tunnel metadata request
  still fits before bandwidth is exhausted.

## Risks / Trade-offs
- [D1 makes registration uninterruptible for its duration] → registration is a
  bounded LP exchange with its own timeouts; the caller's token still aborts
  selection and everything before the exchange.
- [D4 removes a public field] → the crate is unpublished/new; call sites are updated
  in the same change.
- [D2 rejects some responses that carry usable answers] → a truncated answer is
  incomplete by definition; erroring is safer than silently using a partial set.

## Open Questions
- D4: remove vs wire — resolved at implementation time by checking `StackConfig::mtu`
  call sites (expected: only the tunnel sets it redundantly → remove).

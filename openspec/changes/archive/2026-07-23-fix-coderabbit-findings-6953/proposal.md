# Proposal: fix-coderabbit-findings-6953

## Why

CodeRabbit's automated review of PR #6953 surfaced 16 findings on top of the human
review already addressed. Assessed against the current (rebased) branch, most are
already safe or out of scope, but a handful are genuine correctness, robustness, and
doc-accuracy gaps worth fixing before merge — notably a lost-ticket cancellation
window in registration, acceptance of truncated DNS responses, an unbounded
`open_bi()` in the QUIC bridge, a silently-ineffective MTU config knob, an unpinned
QUIC bridge slipping through, and a README that overpromises a kill-switch.

## What Changes

- **Registration is non-cancellable past ticket-spend** (`nym-sdk-session`): the LP
  registration exchange (which spends a WireGuard ticket) no longer runs inside a
  `select!` racing the caller's `CancellationToken`, so cancellation can't drop the
  future after the gateway has processed the spend — mirroring the `ensure_ticketbooks`
  funds-safety fix. Cancellation is checked before the exchange begins.
- **Truncated DNS responses are rejected** (`smol-core`): a response with the `TC`
  bit set no longer returns partial addresses; it surfaces an error (RFC 1035 —
  truncation requires TCP retry, which is out of scope here).
- **QUIC bridge `open_bi()` is bounded** (`nym-smol-dvpn`): the bi-stream open is
  wrapped in the same connect timeout + cancellation as the QUIC handshake, so a
  stalled bridge can't hang `connect()`.
- **Dead MTU config removed/wired** (`smol-core`): `StackConfig::mtu`/`with_mtu` is
  no longer a silently-ineffective knob.
- **QUIC bridge requires a non-empty identity pin** (`nym-sdk-session`): a directory
  entry with a blank `id_pubkey` is not advertised as QUIC-capable.
- **Test hygiene** (`nym-smol-dvpn`): the live-bringup test no longer calls
  `std::process::exit(0)` (which could kill sibling tests); teardown latency is
  already bounded by a timeout on `shutdown()`.
- **Doc accuracy**: the `smol-dvpn` README states an app-scoped (not universal)
  guarantee; the design doc clarifies the bridge SNI/cert-alt-name relationship and
  the bandwidth top-up headroom rationale.

No API additions; these are fixes to existing behavior and docs. Explicitly out of
scope (assessed): the `addrs[0]` panic (already unreachable — `resolve` never returns
`Ok(empty)`), the pre-existing global AGPL allowance in `deny.toml` (a team decision
on develop), the `smolmix` signature test and archived OpenSpec records (other
crate / archived), and two low-value doc/test-strictness nits.

## Capabilities

### Modified Capabilities

- `dvpn-session`: registration must not be cancellable once a ticket has been spent;
  QUIC-bridge selection requires a non-empty identity pin.
- `dvpn-tunnel`: the QUIC bridge connect must honor the timeout/cancellation for the
  bi-stream open as well as the handshake.
- `smol-core-stack`: the DNS resolver must reject truncated responses rather than
  return partial results.

## Impact

- **Crates**: `sdk/rust/nym-sdk-session`, `sdk/rust/smol-dvpn`, `common/smol-core`.
- **Docs**: `sdk/rust/smol-dvpn/README.md`, `docs/design/sdk/smol-dvpn/design.md`.
- **Tests**: `sdk/rust/smol-dvpn/tests/live_bringup.rs`; new DNS-truncation coverage
  in `common/smol-core`.
- All changes land on `feature/nym-sdk-dvpn` (PR #6953); no new dependencies.

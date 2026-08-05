# Design: dvpn-signer-failure-tests

## Context

Diagnosed against live mainnet (2026-07-23): the aggregated
expiration-date-signatures endpoint on `validator.nymtech.net/api` accepts
connections but never responds (curl: `http=000` after 12–25s), while the
sibling endpoints (`master-verification-key`,
`aggregated-coin-indices-signatures`, `partial-expiration-date-signatures`)
respond in <0.3s. The signers behind aggregation are offline long-term — this
is the steady state of a distributed operator deployment, not an incident.

The client-side failure chain, confirmed by on-disk evidence in
`data/zcash-sync/mainnet/creds.db` (master vk: 1 row, coin-index sigs: 1 row,
expiration sigs: 0 rows, ticketbooks: 0 rows, 507 KB uncheckpointed WAL):

1. `BandwidthController::store_ticketbook`
   (`common/bandwidth-controller/src/controller.rs:524`) runs, in order:
   ensure master vk → ensure coin-index sigs → **ensure expiration-date sigs
   → hangs** → `insert_issued_ticketbook` never reached.
2. The fetch runs inline on the controller's single run-loop task (via
   `on_fetch_complete`), so the whole controller freezes; parked
   `wait_for_ticketbooks` callers never resolve → `ensure_ticketbooks` hangs.
3. No layer has a request timeout: `EcashApiClient` is built without one
   (`common/client-libs/validator-client/src/coconut/mod.rs:99`) and
   `query_random_apis_until_success`
   (`common/bandwidth-fetcher/src/public_data.rs:124`) awaits each API
   unboundedly.
4. Because the ticketbook was never persisted, ctrl-C + retry re-deposits
   real NYM for a new ticketbook and hangs again.

Two facts make an SDK-confined fix both possible and sufficient:

- `store_ticketbook` is **best-effort per step**: an `Err` from any
  ensure-step is logged and the remaining steps still run — including the
  ticketbook insert. Only a *hang* prevents persistence. Converting the hang
  into a bounded error is therefore enough to persist the ticketbook.
- Readiness (`build_readiness_snapshot`, controller.rs:581) is computed from
  **stored** usable ticketbooks only — it does not require expiration-date
  signatures. Once the ticketbook persists, `wait_for_ticketbooks` resolves
  `Ready` and provisioning returns `Ok`. Missing signatures are back-filled
  later by `ensure_global_data` / at spend time, so recovery is automatic
  when signers return.

The injection seam: `BandwidthController::with_credential_fetcher` registers
one object as both `credential_fetcher` and `public_data_fetcher`
(controller.rs:113–118). Both traits (`CredentialFetcher`,
`CredentialPublicDataFetcher`) are public re-exports of
`nym_bandwidth_controller`, and the controller stores `Arc<dyn ...>`, so a
decorator needs no `Clone`. `nym-sdk-session` already constructs the
`NyxdCredentialFetcher` it passes in (`session.rs:324`), so it can wrap it.

Test-infrastructure precedent:
`common/bandwidth-controller/tests/managed_ticket_types.rs` already drives the
real controller with a hand-written fetcher implementing both traits plus
`initialise_ephemeral_storage()` — no network, no sqlite. The only missing
piece for an end-to-end test is fabricating an `IssuedTicketBook`
(no fixture exists; the real issuance path in
`common/credentials/src/ecash/bandwidth/issuance.rs` shows the compact-ecash
calls needed: `generate_keypair_user_from_seed`, `withdrawal_request`,
`issue_verify`, `aggregate_wallets`).

## Goals / Non-Goals

**Goals:**

- Provisioning never hangs indefinitely on unresponsive signers: it either
  succeeds (ticketbook persisted, signatures deferred) or fails fast with a
  distinct, actionable error.
- A paid-for ticketbook is never lost: once issuance succeeds, the ticketbook
  reaches storage even when global signing data is unavailable, and a retry
  never re-deposits.
- All of the above is provable deterministically in CI — no mainnet, no
  sandbox, no wall-clock waits, no NYM spent.
- Zero changes to `common/` production code; everything lands in
  `sdk/rust/nym-sdk-session`.

**Non-Goals:**

- Making ticketbooks *spendable* while signers are below threshold —
  expiration-date signatures are cryptographically required to spend; with
  this change a spend attempt fails fast instead of hanging, and works the
  moment signers recover. (Client-side partial-signature aggregation is a
  separate proposal: `dvpn-signer-fault-http-harness` covers its
  fault-injection prerequisite.)
- Fixing the mainnet aggregation endpoint or the offline signers.
- Hardening other consumers of `common/bandwidth-controller` (gateways,
  credential proxy, mixnet clients) — they would need the same decorator or a
  future `common/`-level fix, out of scope here.
- Restructuring the controller so fetches don't run on its run-loop task
  (a worthwhile `common/` improvement, but not SDK-confined).

## Decisions

### D1. Timeout as a fetcher decorator in the SDK, not a `common/` change

A `TimeoutFetcher<F>` in `nym-sdk-session` wraps `NyxdCredentialFetcher` and
is passed to `with_credential_fetcher` unchanged.

- Alternative: add a timeout to `EcashApiClient` construction
  (`validator-client`) or to `query_random_apis_until_success`
  (`bandwidth-fetcher`). Rejected for this change: those crates back every
  credential-consuming component in the monorepo; a behavior change there
  needs ecash-owner review and regression testing far beyond the dVPN SDK.
  The decorator gets the dVPN SDK safe now and stands as a reference
  implementation if the `common/` fix is later adopted.
- The decorator implements both traits by delegation; only the three
  read-only public-data fetches get `tokio::time::timeout`.

### D2. `fetch_ticketbooks` is never timed

Deposit + issuance moves funds on-chain. Aborting the future mid-deposit
could strand a deposit that the fetcher's pending-request store would
otherwise recover on a later fetch — and the observed failure mode is not in
this call anyway. Cancellation of the overall operation remains the caller's
cancellation token, which the controller/fetcher already handle funds-safely.

### D3. Timeout value: constant, generous default (15s per call)

Sibling endpoints answer in <0.3s; a healthy aggregation call is similar. 15s
is ~50× the healthy latency yet turns an infinite hang into a bounded delay
of at most ~45s across the three calls (they run sequentially in
`store_ticketbook`). Not exposed in `SessionConfig` initially — no known
caller needs to tune it, and adding config later is backward-compatible.
Alternative (rejected for now): per-call budget in `SessionConfig` — adds API
surface without a driving use case.

### D4. Timeout error is a first-class `FetcherError`

The decorator maps `Elapsed` to a dedicated error type implementing
`nym_bandwidth_controller::FetcherError` (kind: transient/`Other`), so the
controller's existing best-effort logging and `FetchFailed` readiness
reporting carry a message that names unresponsive signers, not a generic
timeout.

### D5. Outer provisioning timeout in `ensure_ticketbooks` (defense in depth)

`Session::ensure_ticketbooks` already races cancellation in a
`tokio::select!` (session.rs:373); add a `tokio::time::timeout` around the
work arm mapping to a new `SessionError::ProvisioningTimeout` with a message
that names the likely cause (unresponsive ecash signers). Budget: generous
multiple of the per-call timeout plus deposit time (e.g. 5 minutes) — it
exists to catch *unforeseen* stalls, not to race D1. Cancellation-safety
argument is unchanged from the existing select: the deposit runs in the
controller task and interrupted issuance is recovered from the
pending-request store.

### D6. Tier 1 tests: virtual-clock unit tests of the decorator

`#[tokio::test(start_paused = true)]` + `tokio::time::advance`. Inner-fetcher
modes: hang (`std::future::pending()`), slow-under-threshold (must succeed),
slow-over-threshold (must error), immediate error (passes through), and
`fetch_ticketbooks` hang (must NOT be timed — asserted by advancing past the
per-call timeout and observing the future still pending). Deterministic,
sub-second, no network.

### D7. Tier 2 tests: real controller + ephemeral storage + mock fetcher

Integration tests in `sdk/rust/nym-sdk-session/tests/`, modeled on
`managed_ticket_types.rs`:

- A `FlakyFetcher` with a mode enum — `Hang`, `Slow(Duration)`, `Error`,
  `Partial` (vk + coin-index Ok, expiration fails) — and an atomic
  `fetch_ticketbooks` call counter.
- Bug reproduction: mode `Hang` **without** the decorator → assert
  `wait_for_ticketbooks` does not resolve within a deadline and storage holds
  0 ticketbooks (the exact mainnet `creds.db` state).
- Recovery: mode `Hang`/`Partial` **with** the decorator → assert the
  ticketbook is in storage and `wait_for_ticketbooks` resolves `Ok`.
- Money safety: run provisioning twice against the same storage; assert the
  second run makes zero additional `fetch_ticketbooks` calls.
- A shared test-support module fabricates a threshold-signed
  `IssuedTicketBook` using the compact-ecash primitives (mirroring
  `issuance.rs`); written once, reusable by future tests.

Trade-off: these tests exercise `common/` code paths from the SDK crate. That
is deliberate — the claim under test is "the SDK survives signer failure
through the real controller", and the controller's public API is the
supported surface.

### D8. Tests live in `nym-sdk-session`, not `common/bandwidth-controller`

Keeps the change SDK-confined per the stated constraint, and the decorator
being tested lives there. If the ecash owners later adopt a `common/`-level
timeout, the Tier 2 suite ports trivially (it only uses public APIs).

## Risks / Trade-offs

- [Ticketbook persisted but unspendable until signers return] → Explicitly
  accepted: readiness reports Ready while a spend would still need
  expiration-date signatures; the spend path (`prepare_ecash_ticket_inner`)
  will attempt the same fetch — through the decorator it fails in ≤15s with a
  clear error instead of hanging. Documented in the spec delta as degraded
  behavior.
- [15s × 3 sequential fetches adds up to ~45s worst-case per stored
  ticketbook while signers are down] → Bounded and logged; vastly better than
  infinite. Tunable later via config if it matters.
- [Decorator only protects the dVPN SDK; other controller consumers still
  hang] → Out of scope by constraint; the diagnosis and decorator serve as
  the template for a reviewed `common/` fix.
- [Fabricating an `IssuedTicketBook` couples tests to compact-ecash
  internals] → Mitigated by mirroring the real issuance path
  (`issuance.rs`) and isolating construction in one helper; if the primitives
  change, one function changes.
- [Tier 2 hang-reproduction test asserts a negative (does not resolve within
  deadline)] → Use a short real deadline (e.g. 2s) with paused-clock control
  where possible; the assertion is "still pending after generous grace", the
  inverse assertions (recovery cases) are positive and airtight.
- [Outer timeout (D5) could fire during a legitimately slow but healthy
  deposit] → Budget set well above observed healthy end-to-end issuance;
  error message tells the caller funds are recoverable via the
  pending-request store on retry.

## Migration Plan

Additive, SDK-internal; no API breakage (`Session` construction is
unchanged; one new `SessionError` variant is added, which is non-exhaustive
for callers matching on it — verify `SessionError` is `#[non_exhaustive]` or
accept the minor source-compat note in the changelog). No data migration:
existing credential stores work as-is; stores stranded by the old hang (0
ticketbooks) simply provision fresh on the next run — which now persists.
Rollback = revert the commit.

## Open Questions

- Should the per-call timeout be surfaced in `SessionConfig` from day one?
  (Default: no — add when a caller needs it.)
- `SessionError` non-exhaustiveness: confirm during implementation whether
  adding `ProvisioningTimeout` is source-compatible for existing matchers.
- Exact outer-timeout budget (D5): pick after measuring healthy sandbox
  issuance end-to-end in CI.

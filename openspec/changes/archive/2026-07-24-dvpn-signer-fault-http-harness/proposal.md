# Proposal: dvpn-signer-fault-http-harness

## Why

The companion change `dvpn-signer-failure-tests` proves signer-failure
tolerance at the fetcher-trait seam, but it substitutes a mock for the entire
real fetch stack — `NyxdGlobalDataFetcher`, its ecash-API discovery/caching,
`query_random_apis_until_success`, and the HTTP client whose missing timeout
is the actual root cause. A defect in any of those layers (e.g. a retry loop
that serializes hangs across N discovered APIs, connection-pool exhaustion,
DNS behavior) is invisible to trait-level mocks. Mainnet demonstrated exactly
such an HTTP-level failure (an endpoint that accepts connections and never
responds); reproducing it against the real client stack in CI is the only way
to validate fixes at full fidelity — and it is the prerequisite for
evaluating the future client-side partial-signature aggregation idea, which
depends on per-signer HTTP behavior under partial outage. This proposal is
deliberately separable: it may not get implemented if the trait-level suite
proves sufficient in practice.

## What Changes

- Add an HTTP-level fault-injection test harness (dev-only, behind a test
  crate or `#[cfg(test)]` support module) that stands up local mock nym-api
  servers (`wiremock` or equivalent) serving the real ecash routes:
  `/v1/ecash/master-verification-key`,
  `/v1/ecash/aggregated-coin-indices-signatures`,
  `/v1/ecash/aggregated-expiration-date-signatures`,
  `/v1/ecash/partial-expiration-date-signatures` — each independently
  scriptable as healthy / hanging (accept, never respond) / slow / erroring,
  mirroring the observed mainnet behavior.
- Provide a fake ecash-API discovery source so the real
  `NyxdGlobalDataFetcher` resolves its `EcashApiClient` list to the mock
  servers without a chain: either a small `DkgQueryClient` test double
  returning fabricated verification-key shares that announce the mock URLs,
  or a thin constructor/seam for injecting a prebuilt client list (whichever
  proves feasible without touching `common/` production code — to be settled
  in design).
- Add integration tests that drive the REAL `NyxdGlobalDataFetcher` through
  the harness: hang on aggregated-expiration-date-signatures with healthy
  siblings (the exact mainnet signature), multi-API fallback behavior when
  the first discovered API hangs, and (if the SDK timeout decorator from the
  companion change is layered on top) bounded end-to-end latency.
- Add a signer-threshold probe scenario: N mock signers with only K alive
  serving partials — measuring what the current stack does, as groundwork for
  the client-side partial aggregation decision.
- No changes to `common/` production code; any injection seam that cannot be
  achieved from outside is reported back as a finding rather than forced in.

## Capabilities

### New Capabilities

- `dvpn-signer-fault-harness`: an HTTP-level fault-injection harness and test
  suite that reproduces real-world ecash signer/nym-api failure modes
  (hang, slow, error, partial availability) against the unmodified
  `NyxdGlobalDataFetcher` HTTP stack, deterministically and offline.

### Modified Capabilities

_None — the harness observes and validates; it changes no shipped behavior._

## Impact

- **Code**: new dev-only test module/crate under `sdk/rust/nym-sdk-session`
  (or a sibling `tests/` support directory); no production code paths change.
- **Dependencies**: dev-deps on `wiremock` (or `axum`/`hyper` used directly
  for hang semantics — wiremock's ability to hold a connection open
  indefinitely must be verified in design), plus existing
  `nym-bandwidth-fetcher` / `nym-validator-client` test surfaces.
- **CI**: adds local-socket integration tests; no network egress, no chain,
  no funds. Runtime bounded by injected timeouts (seconds).
- **Risk of non-implementation**: explicitly acceptable — the companion
  trait-level suite already guards the regression; this harness raises
  fidelity and unblocks the partial-aggregation investigation. If the
  discovery-injection seam turns out to require `common/` changes, the change
  is paused and the finding handed to the ecash owners instead.

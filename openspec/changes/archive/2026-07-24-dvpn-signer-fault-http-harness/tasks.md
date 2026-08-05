# Tasks: dvpn-signer-fault-http-harness

## 1. Feasibility spikes (fail fast on the two unknowns)

- [x] 1.1 Spike: verify `wiremock` can hold an accepted connection open
      indefinitely without responding; if not, confirm the hand-rolled
      hyper/tokio listener approach (D2) with a 20-line prototype
- [x] 1.2 Spike: implement the minimal `DkgQueryClient` double
      (`query_dkg_contract` only) and confirm
      `NyxdGlobalDataFetcher::new(fake_dkg)` + one discovery round-trip
      resolves fabricated `ContractVKShare`s (valid bs58
      `VerificationKeyAuth`, `verified: true`, mock URL announce address)
      into usable `EcashApiClient`s — without touching `common/`

## 2. Harness core

- [x] 2.1 Implement the mock nym-api server: per-route `FaultMode`
      (`Healthy(body)`, `Slow(Duration)`, `Error(status)`, `Hang`) with
      runtime switching and teardown that releases parked `Hang` requests
- [x] 2.2 Generate healthy fixture bodies for the four ecash routes from the
      shared compact-ecash fixture helper (reuse/extend the companion
      change's test-support module) so responses parse in the real client
- [x] 2.3 Implement `FakeDkg` (from spike 1.2) productionized for tests:
      vk-shares, epoch, and threshold queries answered; unexpected queries
      fail loud
- [x] 2.4 Harness builder API: N servers × per-route modes, returning the
      `FakeDkg` + server handles; every constructed test wrapped in an outer
      test-side timeout

## 3. Characterization and guard tests

- [x] 3.1 Characterization: unmodified `NyxdGlobalDataFetcher` vs. hanging
      aggregated-expiration-date-signatures with healthy siblings — request
      still pending at the characterization deadline (reproduces the mainnet
      signature offline); name prefixed `characterize_`
- [x] 3.2 Guard: fetcher wrapped in the SDK `TimeoutFetcher` (companion
      change) against the same fault — returns a timeout error within the
      configured bound, measured through the real HTTP stack
- [x] 3.3 Multi-API: several discovered servers, first hanging + one healthy,
      per-call bounds active — fetch succeeds via the healthy API within
      (per-call budget × failed attempts); characterize the unbounded
      variant for the record
- [x] 3.4 Sibling-route sanity: healthy-everything run fetches and parses vk,
      coin-index, and expiration-date signatures through the real stack
      (validates fixtures, not just faults)

## 4. Partial-availability threshold probe

- [x] 4.1 Scenario builder: N signers, K serving valid
      partial-expiration-date-signatures, aggregated route dead everywhere,
      threshold from `FakeDkg`
- [x] 4.2 Probe test recording which partial fetches succeed and their
      latency for K ≥ threshold and K < threshold; assertion-light (data
      collection for the client-side aggregation go/no-go)

## 5. Verification & wrap-up

- [x] 5.1 `cargo test -p nym-sdk-session` and `cargo clippy --tests` green;
      total harness-suite wall clock under ~10s
- [x] 5.2 Confirm `git diff --stat` touches only `sdk/rust/nym-sdk-session`
      tests/dev-deps and openspec files (no `common/` production code)
- [x] 5.3 Document the harness in the test-support module rustdoc: fault
      modes, discovery double, characterization-vs-guard convention, and the
      threshold-probe purpose
- [x] 5.4 If any spike fails on a `common/` blocker: stop, record the blocker
      and findings in this change's design.md, and hand off to the ecash
      owners instead of forcing a seam

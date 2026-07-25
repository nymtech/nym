# Design: dvpn-signer-fault-http-harness

## Context

The companion change (`dvpn-signer-failure-tests`) validates signer-failure
tolerance at the `CredentialPublicDataFetcher` trait seam. Everything below
that seam is mocked away — yet the mainnet root cause lives below it: an HTTP
endpoint (`/v1/ecash/aggregated-expiration-date-signatures`) that accepts
connections and never responds, awaited by a client with no timeout
(`EcashApiClient` built without one,
`common/client-libs/validator-client/src/coconut/mod.rs:99`) inside a
try-each-API loop with no bound
(`query_random_apis_until_success`,
`common/bandwidth-fetcher/src/public_data.rs:124`).

The real stack under test: `NyxdGlobalDataFetcher` discovers ecash APIs via
`all_ecash_api_clients` → `DkgQueryClient::get_all_verification_key_shares`
→ `ContractVKShare` → `EcashApiClient` (URL parsed from `announce_address`,
share must be `verified` and carry a valid bs58 `VerificationKeyAuth`),
caches them 30 min (`EcashApiClientsCache`), then queries the routes in
random order until one succeeds.

Feasibility facts established by code inspection:

- `DkgQueryClient` has exactly **one required method**
  (`query_dkg_contract`); all discovery entry points are default methods over
  it. A test double therefore implements one function that answers the
  vk-shares query with fabricated shares announcing local mock-server URLs.
- `NyxdGlobalDataFetcher::new(client)` is generic over `C: DkgQueryClient` —
  the double injects without any `common/` change.
- `ContractVKShare.share` must parse as a real `VerificationKeyAuth`: the
  companion change's compact-ecash fixture helper (keygen) supplies valid
  keys.
- The mainnet failure signature to reproduce is protocol-level "accept, then
  silence" — the mock server must be able to hold an accepted connection
  open indefinitely without writing a response.

## Goals / Non-Goals

**Goals:**

- Reproduce, offline and deterministically, the exact observed mainnet
  failure (hang on aggregated-expiration-date-signatures; healthy siblings)
  against the **unmodified** `NyxdGlobalDataFetcher` + HTTP client stack.
- Characterize multi-API behavior: N discovered APIs where the first K hang
  — does the loop serialize hangs? (Today: yes, unboundedly; the harness
  turns that from an inference into a measured fact and guards any future
  fix.)
- Provide the measurement bed for the client-side partial-signature
  aggregation decision: K-of-N signers alive serving
  `partial-expiration-date-signatures`, threshold known, aggregated route
  dead.
- Zero `common/` production-code changes; dev-only additions.

**Non-Goals:**

- Implementing client-side partial aggregation itself (future change,
  informed by this harness).
- Testing chain interaction, deposits, or `fetch_ticketbooks` (covered at
  the trait level by the companion change; involves funds and nyxd).
- Replacing the companion change's trait-level tests — those stay as the
  fast regression guard; this harness is the high-fidelity layer.
- CI-gating the whole repo on this suite initially — it lands as a normal
  `cargo test` target of the SDK crate.

## Decisions

### D1. Discovery injection via a single-method `DkgQueryClient` double

`FakeDkg` implements `query_dkg_contract`, pattern-matching the query enum to
answer: vk-shares (fabricated `ContractVKShare`s, `verified: true`,
`announce_address` = mock server URL, `share` = bs58 of a generated
`VerificationKeyAuth`), threshold, and epoch queries; anything unexpected
returns an error (fail-loud). Alternatives rejected:
- Constructing `EcashApiClient`s directly (fields are `pub`) — no seam on
  `NyxdGlobalDataFetcher` accepts a prebuilt list
  (`new_with_ecash_clients` is `pub(crate)`), and adding one would touch
  `common/`.
- Faking at DNS/proxy level — needless complexity for local mocks.

### D2. Mock server: hand-rolled hyper/tokio listener, not wiremock-first

The critical behavior is **accept-then-never-respond**, per-route,
switchable at runtime. A small `tokio::net::TcpListener` + `hyper` service
(~100 lines) gives exact control: route table mapping each ecash path to a
`FaultMode` (`Healthy(body)`, `Hang`, `Slow(d)`, `Error(status)`), where
`Hang` parks the request future on a `Notify` that only test teardown fires.
`wiremock` is evaluated first during implementation (it may support
indefinite delays cleanly); if it does, prefer it — but the design does not
depend on it. Bodies for `Healthy` come from the same compact-ecash fixture
generation as the companion change (signatures that parse, epoch-consistent).

### D3. Suite composition: characterization tests + guard tests

Two categories, explicitly labeled:
- **Characterization** (assert current behavior, however undesirable): the
  raw fetcher against a hanging aggregated route does not return within N
  seconds — proving the harness reproduces mainnet and documenting the
  unfixed stack. Bounded by an outer test-side `tokio::time::timeout` so the
  suite itself never hangs.
- **Guard** (assert desired behavior of fixed configurations): the fetcher
  wrapped in the companion change's `TimeoutFetcher` completes within the
  bound; multi-API fallback reaches the healthy API once per-call bounds
  exist.
This split keeps the suite honest if/when a `common/`-level timeout ships:
characterization tests get updated deliberately, guards keep passing.

### D4. Threshold-probe scenario as data collection, not assertion

Spin up N mock signers, K serving valid partials, aggregated route dead
everywhere; run a probe (initially just the raw HTTP calls the future
partial-aggregation client would make) and record which K suffice per the
DKG threshold from `FakeDkg`. Output feeds the go/no-go decision on
client-side aggregation. Kept assertion-light so it does not rot.

### D5. Location: `sdk/rust/nym-sdk-session/tests/` + shared `tests/support/`

Same crate as the companion change's Tier 2 suite, sharing the fixture
helper (ecash keygen, signature fabrication). A separate harness crate was
rejected: nothing else consumes it yet, and promotion later is mechanical.

### D6. Real-clock tests with tight budgets, not virtual clock

The system under test includes a real TCP stack; `start_paused` does not
virtualize socket I/O. Timeouts under test are set small via the decorator's
constructor (already parameterized in the companion change), keeping the
whole suite under a few seconds of wall clock.

## Risks / Trade-offs

- [`DkgQueryMsg`/`ContractVKShare` shapes shift under us] → The double lives
  in one file; compile errors localize the fix. Fail-loud on unexpected
  queries surfaces silent contract-protocol drift immediately.
- [Hang semantics differ from mainnet middleboxes (e.g. mainnet may sit
  behind a proxy that behaves differently at TCP level)] → We reproduce the
  observed client-visible behavior (headers accepted, no response); finer
  distinctions (RST vs FIN vs silence) can be added as `FaultMode` variants
  if ever needed.
- [Characterization tests encode "the stack hangs" and will fail the day a
  `common/` timeout ships] → Intentional: that failure is the signal to
  flip them to guards. Labeled in test names (`characterize_`).
- [Duplicated fidelity with companion change → maintenance drag] → Scope
  discipline: this suite only covers what trait mocks cannot (HTTP layer,
  discovery, multi-API loop, partial-signature groundwork). Anything
  expressible at the trait seam belongs to the companion suite.
- [May never be implemented] → Acceptable by explicit stakeholder decision;
  the change is self-contained and the proposal stands as the recorded plan.

## Migration Plan

Dev-only, additive; no production or data migration. Land in one PR; if the
discovery double proves impossible without `common/` changes (contra the
code inspection above), stop, document the blocker in this change, and hand
the finding to the ecash owners.

## Implementation Findings (2026-07-24)

Recorded per task 5.4's stop-and-document rule — no `common/` blockers were
hit; both spikes passed. Findings that feed future work:

- **D2 resolved: hand-rolled, no wiremock.** The `Hang` mode is ~10 lines on
  a raw `TcpListener`; no new dependency was added at all.
- **D1 confirmed with one adjustment:** `ContractVKShare.owner` must be
  syntactically valid bech32 (discovery parses it into a cosmrs `AccountId`),
  so the double uses real-format `n1…` addresses. Everything else worked as
  designed: one `query_dkg_contract` method, serde round-trip responses.
- **Multi-API finding (the harness's first real yield):** the SDK's
  `TimeoutFetcher` bound wraps the fetcher's WHOLE try-each-API loop, so a
  single call that shuffles a hanging API first times out without ever
  reaching a healthy one — per-API fallback requires a per-request bound
  inside `query_random_apis_until_success`
  (`common/bandwidth-fetcher/src/public_data.rs`). What holds today, and is
  guarded by the tests: every attempt is bounded, and bounded retries reach a
  healthy signer. This is the concrete motivation to hand the ecash owners
  for the `common/`-level timeout.
- **Threshold probe (go-signal):** with the aggregated route dead on every
  signer and K ≥ threshold serving partials, sufficient partial
  expiration-date signatures are retrievable directly (single-digit ms on
  loopback) — client-side partial aggregation is viable whenever a signer
  quorum is alive, exactly the situation observed on mainnet. The K <
  threshold case correctly reports insufficiency.

## Open Questions

- Does `wiremock` support indefinite response delay cleanly (D2), or do we
  hand-roll from the start?
- Should the threshold-probe (D4) target the real
  `partial-expiration-date-signatures` response schema now, or wait for the
  partial-aggregation change to define its client?
- Suite placement in CI: default `cargo test -p nym-sdk-session` (current
  plan) vs. an opt-in feature flag if runtime grows.

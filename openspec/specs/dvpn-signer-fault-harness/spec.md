# dvpn-signer-fault-harness Specification

## Purpose
TBD - created by archiving change dvpn-signer-fault-http-harness. Update Purpose after archive.
## Requirements
### Requirement: HTTP-level ecash fault-injection harness

The test suite SHALL provide a local, offline fault-injection harness that
stands up mock nym-api servers serving the ecash routes
(`master-verification-key`, `aggregated-coin-indices-signatures`,
`aggregated-expiration-date-signatures`,
`partial-expiration-date-signatures`), where each route on each server is
independently configurable as healthy (valid, parseable fixture body), slow,
erroring (HTTP error status), or hanging — accepting the connection and
never writing a response. The harness SHALL make the unmodified
`NyxdGlobalDataFetcher` discover these mock servers through its real
discovery path, via a `DkgQueryClient` test double answering the
verification-key-share query with fabricated shares that announce the mock
server URLs and carry valid verification keys. The harness MUST NOT require
changes to `common/` production code, network egress, chain access, or
funds, and every test using it MUST be bounded by a test-side timeout so
the suite never hangs even when reproducing hang faults.

#### Scenario: Reproduce the observed mainnet failure signature

- **WHEN** the harness serves healthy master-verification-key and
  coin-index-signatures routes but a hanging
  aggregated-expiration-date-signatures route, and the unmodified fetcher
  stack requests expiration-date signatures
- **THEN** the request does not complete within the characterization
  deadline, demonstrating offline the same client-visible behavior observed
  against mainnet

#### Scenario: Real discovery resolves to mock servers

- **WHEN** `NyxdGlobalDataFetcher` performs ecash-API discovery against the
  harness's `DkgQueryClient` double
- **THEN** it builds its API client list from the fabricated shares and
  issues its HTTP requests against the local mock servers

#### Scenario: Bounded behavior with the timeout decorator layered on

- **WHEN** the fetcher is wrapped in the SDK's timeout decorator and a
  route hangs
- **THEN** the fetch returns a timeout error within the configured bound,
  measured end-to-end through the real HTTP stack

### Requirement: Multi-API fallback characterization

The harness SHALL support multiple simultaneously discovered mock APIs with
independent per-server fault modes, so the fetcher's try-each-API loop is
exercised as deployed: tests SHALL characterize what happens when the first
discovered API(s) hang while a later one is healthy, and SHALL guard that,
with per-call bounds in place, the loop reaches the healthy API and
succeeds within a bounded total time.

#### Scenario: Hanging first API with a healthy fallback

- **WHEN** discovery returns several APIs, at least one hanging and at least
  one healthy, with per-call timeout bounds active
- **THEN** the fetch ultimately succeeds via the healthy API and total
  latency is bounded by the per-call budget times the number of failed
  attempts

### Requirement: Partial-availability threshold probe

The harness SHALL support a K-of-N signer scenario — N discovered mock
signers of which only K serve valid partial expiration-date signatures, the
aggregated route unavailable everywhere, and the DKG threshold reported by
the discovery double — to collect the ground truth needed to decide whether
client-side partial-signature aggregation could restore spendability during
partial signer outages.

#### Scenario: K live signers at or above threshold

- **WHEN** K ≥ threshold mock signers serve valid partials and the
  aggregated route hangs on all servers
- **THEN** the probe records that sufficient partials are retrievable for
  client-side aggregation, and records which requests succeeded and how
  long they took


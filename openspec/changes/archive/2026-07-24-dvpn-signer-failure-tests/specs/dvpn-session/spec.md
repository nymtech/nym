# Delta: dvpn-session — signer-failure tolerance

## ADDED Requirements

### Requirement: Tolerance to unresponsive ecash signers

`nym-sdk-session` SHALL treat unresponsive or partially available ecash
signers/nym-apis as a normal operating condition, not an error to hang on.
Every read-only global-signing-data fetch performed on the session's behalf
(master verification key, coin-index signatures, expiration-date signatures)
SHALL be bounded by a per-call timeout, converting an endpoint that accepts a
connection but never responds into a bounded fetch error. The
ticketbook-issuance call (which deposits funds on-chain) MUST NOT be subject
to this timeout — interrupted issuance remains governed by the existing
cancellation-safety and pending-request recovery guarantees. A ticketbook
that has been successfully issued SHALL be persisted to the credential store
even when the global signing data required to spend it cannot currently be
fetched; the missing signing data SHALL be fetched later (during background
global-data reconciliation or at spend time) without re-issuing the
ticketbook. Provisioning (`ensure_ticketbooks`) SHALL additionally be bounded
by an overall timeout and surface a distinct session error identifying
unresponsive signers as the likely cause, rather than blocking indefinitely.

#### Scenario: Public-data fetch against a hung endpoint fails fast

- **WHEN** an ecash endpoint serving expiration-date signatures accepts the
  connection but never responds
- **THEN** the session-side fetch returns a timeout error within the
  configured per-call bound instead of hanging, and the error identifies the
  fetch that timed out

#### Scenario: Issued ticketbook is persisted despite missing signing data

- **WHEN** a ticketbook has been issued (funds deposited, wallet aggregated)
  and the subsequent expiration-date-signatures fetch fails or times out
- **THEN** the ticketbook is stored in the credential store, provisioning
  completes with the ticketbook counted as stocked, and the missing
  signatures are fetched later without a new deposit

#### Scenario: Retry after signer outage never re-purchases

- **WHEN** provisioning runs again over a credential store already holding a
  usable ticketbook that was persisted during a signer outage
- **THEN** no new issuance (and no deposit) is requested for that ticket type

#### Scenario: Provisioning surfaces a timeout instead of hanging

- **WHEN** the provisioning path stalls for longer than the overall
  provisioning budget for any reason
- **THEN** `ensure_ticketbooks` returns a distinct provisioning-timeout error
  naming unresponsive ecash signers as the likely cause, and any deposit
  already made remains recoverable from the pending-request store

#### Scenario: Deposit is never aborted by the fetch timeout

- **WHEN** the issuance call (deposit + wallet aggregation) is slow but
  progressing
- **THEN** the per-call public-data timeout does not cancel it; only caller
  cancellation or the overall provisioning budget applies, both preserving
  the funds-recovery guarantees

#### Scenario: Signer failure modes are reproducible in CI

- **WHEN** the test suite simulates signer failure (hanging fetch, slow
  fetch, erroring fetch, or partial availability where only
  expiration-date signatures fail) against the real bandwidth controller and
  an ephemeral credential store
- **THEN** the suite deterministically demonstrates, without network access
  or funds: the pre-fix hang and empty store, the post-fix persisted
  ticketbook and resolved readiness, and the zero-re-deposit retry property

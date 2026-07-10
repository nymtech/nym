# dvpn-session Specification (delta)

## MODIFIED Requirements

### Requirement: Mnemonic-funded zk-nym ticketbook issuance

`nym-sdk-session` SHALL, from a caller-supplied mnemonic, deposit NYM on-chain and
obtain issued zk-nym ticketbooks from the Nym API signers, reusing
`nym-bandwidth-controller`/`nym-bandwidth-fetcher`. It SHALL support the wireguard
ticket types `V1WireguardEntry` and `V1WireguardExit`, and it MUST NOT deposit for
any other ticket type (enforced by scoping the controller's
`managed_ticket_types` to the WireGuard types and only ever requesting those). The
session SHALL run the bandwidth controller
event loop and perform all ticket spending through its request sender (the
single-writer pattern); a caller already running a controller MAY supply its own
`BandwidthTicketProvider` instead, in which case the session spawns no controller.
The client id used for issuance SHALL be derived from a hash of the mnemonic
entropy (never the raw entropy), and owned mnemonic material SHALL be zeroized on
drop.

#### Scenario: Issue wireguard ticketbooks
- **WHEN** a caller provides a funded mnemonic and requests dVPN access
- **THEN** the session deposits NYM, contacts the signers, and obtains
  `V1WireguardEntry`/`V1WireguardExit` ticketbooks

#### Scenario: Setup phase is abortable without losing funds
- **WHEN** the caller cancels the provided cancellation/shutdown token during
  deposit/issuance
- **THEN** setup stops, no tunnel setup proceeds, and any deposit already made is
  recorded in the pending-requests store and recovered on a later fetch

#### Scenario: Mixnet ticket types are never purchased
- **WHEN** a session provisions, restocks, or tops up in any mode
- **THEN** no deposit is ever made for a non-WireGuard ticket type

#### Scenario: External provider preserves single writer
- **WHEN** the caller supplies an externally managed `BandwidthTicketProvider`
- **THEN** the session uses it for all spending and does not spawn its own
  controller over the credential store

### Requirement: Persistent credential storage for reuse

Issued ticketbooks SHALL be persisted in a credential store (`nym-credential-storage`)
so a subsequent tunnel bring-up reuses stocked tickets without re-depositing. A
storage failure while checking for stored ticketbooks SHALL surface as a distinct
storage error and MUST NOT be treated as "no ticketbook stored" (which would
trigger a redundant paid issuance).

#### Scenario: Second bring-up reuses stored tickets
- **WHEN** a tunnel is brought down and later brought up again with the same store
- **THEN** the session uses already-stored ticketbooks and skips a new deposit if
  sufficient bandwidth remains

#### Scenario: Storage failure does not trigger re-issuance
- **WHEN** the credential store errors while checking for a usable ticketbook
- **THEN** the session fails with a storage error instead of depositing for a new
  ticketbook

## ADDED Requirements

### Requirement: Opt-in automatic chain-side restock

Automatic on-chain ticketbook restocking SHALL be disabled by default. The session
SHALL offer an opt-in (e.g. `with_automatic_topups(policy)`) whose policy sets the
restock thresholds (minimum tickets, restock amount, check interval, soon-expiry
horizon); enabling it installs the credential fetcher into the running controller
scoped to the session's WireGuard ticket types only. Without the opt-in, the
session issues only what initial provisioning requires and never deposits in the
background.

#### Scenario: Default session never restocks in the background
- **WHEN** a session is created without the automatic-topups opt-in and its stored
  tickets are gradually spent
- **THEN** no background deposit is ever made; ticket exhaustion is observable so
  the caller can act

#### Scenario: Opted-in session restocks per policy
- **WHEN** automatic topups are enabled and the stored WireGuard tickets fall below
  the policy threshold
- **THEN** the controller deposits and issues new ticketbooks of the needed
  WireGuard type(s) without caller involvement

### Requirement: Efficient topology retrieval for selection

A registration that selects multiple gateways SHALL fetch the node topology once
and reuse it across the selections, rather than fetching per selection.

#### Scenario: Two-hop registration fetches topology once
- **WHEN** a two-hop registration selects entry and exit gateways
- **THEN** the described-nodes topology is fetched a single time and both
  selections evaluate against it

### Requirement: dVPN gateway eligibility ignores mixnet role declarations

Gateway eligibility for dVPN SHALL require WireGuard, authenticator, and LP data,
and SHALL NOT be restricted by the node's declared mixnet entry/exit role (that
role distinction applies to mixnet mode only).

#### Scenario: Exit-declared node usable as dVPN entry
- **WHEN** a WireGuard-capable node declares only the mixnet exit role
- **THEN** it is still eligible for selection as a dVPN entry hop

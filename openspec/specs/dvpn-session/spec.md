# dvpn-session Specification

## Purpose
Defines nym-sdk-session responsibilities: mnemonic-funded zk-nym ticketbook issuance, credential storage, gateway selection/registration, and QUIC-bridge entry selection.
## Requirements
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

### Requirement: Gateway selection by identity, country, or random

The session SHALL select gateways by one of: exact identity key, two-letter ISO 3166
country code, or uniformly random, filtered to nodes that support the required
WireGuard role.

#### Scenario: Select by identity key
- **WHEN** the caller specifies a gateway identity key
- **THEN** the matching gateway is selected, or an error is returned if not found or
  it lacks WireGuard support

#### Scenario: Select by country code
- **WHEN** the caller specifies a two-letter country code
- **THEN** a WireGuard-capable gateway located in that country is chosen (randomly
  among matches), or an error is returned if none match

#### Scenario: Select randomly
- **WHEN** the caller requests a random gateway
- **THEN** a WireGuard-capable gateway is chosen uniformly at random from the eligible
  set

#### Scenario: Two-hop selects distinct gateways
- **WHEN** a two-hop tunnel is registered
- **THEN** the entry gateway is excluded from the exit selection so the two hops never
  resolve to the same gateway; if the exit spec can only match the entry gateway,
  registration fails with a distinct-gateways error

### Requirement: Gateway registration producing WireGuard configuration

The session SHALL register selected gateways via `nym-registration-client` and return,
per hop, a WireGuard configuration containing the gateway public key, the
LP-negotiated preshared key, the endpoint, and the assigned tunnel IPs. Single-hop
(`gateway=…`) SHALL register a single gateway (LP single-gateway `register_dvpn` path);
two-hop (`entry=…`/`exit=…`) SHALL register both hops.

#### Scenario: Single-hop registration yields one config
- **WHEN** a single gateway is registered for a single-hop tunnel
- **THEN** one WireGuard configuration is returned (gateway public key, negotiated PSK,
  endpoint, assigned IPs) without registering a second hop

#### Scenario: Two-hop registration yields entry and exit configs
- **WHEN** entry and exit gateways are registered for a two-hop tunnel
- **THEN** two WireGuard configurations are returned, each with gateway public key,
  negotiated PSK, endpoint, and assigned IPv4/IPv6

#### Scenario: Registration spends wireguard tickets
- **WHEN** registration completes for an entry/exit hop
- **THEN** the corresponding `V1WireguardEntry`/`V1WireguardExit` ticket is spent from
  the store

### Requirement: Optional dVPN directory for gateway metadata

The session SHALL accept an optional dVPN gateway-directory URL and, when set, fetch
it best-effort (a fetch/parse failure is logged and treated as an empty directory) to
enrich each returned gateway's metadata with its human moniker and, when the described
node omits one, its country. Each hop's returned metadata SHALL include the gateway
identity, directory node id, country, IP, and moniker (when known).

#### Scenario: Monikers populated from the directory
- **WHEN** a directory URL is configured and a selected gateway appears in it
- **THEN** the returned per-hop gateway metadata includes that gateway's moniker

#### Scenario: Directory unavailable is non-fatal
- **WHEN** a directory URL is configured but the fetch or parse fails
- **THEN** the session still provisions and registers gateways, with monikers absent

### Requirement: QUIC-bridge entry gateway selection

The session SHALL provide a two-hop registration that requires the entry gateway to be
QUIC-bridge-capable per the dVPN directory, honoring the entry `GatewaySpec`
(identity / country / random). It SHALL return the entry's QUIC bridge parameters
(bridge addresses, SNI host, base64 ed25519 `id_pubkey`) with the entry hop, and fail
with a distinct error when no QUIC-capable gateway matches. Only the two-hop entry hop
may carry bridge parameters; single-hop and non-QUIC two-hop registrations carry none.

#### Scenario: QUIC entry selected and bridge params returned
- **WHEN** the caller requests a QUIC two-hop registration and a directory gateway
  matching the entry spec advertises a QUIC bridge
- **THEN** that gateway is chosen as entry and its bridge parameters are returned with
  the entry hop

#### Scenario: No QUIC-capable gateway matches
- **WHEN** the caller requests a QUIC entry but no directory gateway matching the spec
  advertises a QUIC bridge (or no directory is configured)
- **THEN** registration fails with a distinct `NoQuicGateway` error

#### Scenario: Non-QUIC registrations carry no bridge params
- **WHEN** a single-hop or non-QUIC two-hop registration completes
- **THEN** no hop carries QUIC bridge parameters

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

### Requirement: Registration is not cancellable once a ticket is spent

Gateway registration SHALL check cancellation before starting the LP registration
exchange, but once the exchange (which spends a WireGuard ticket) has begun it MUST
run to completion or its own failure without being aborted by the caller's
cancellation token — so a cancel cannot drop the future after the gateway has
processed the ticket spend, which would lose the ticket with no configuration
returned. Gateway selection (before any spend) SHALL remain promptly cancellable.

#### Scenario: Cancel before the exchange aborts cleanly
- **WHEN** the cancellation token fires before registration begins the LP exchange
- **THEN** registration returns a cancelled error and no ticket is spent

#### Scenario: Cancel during the exchange does not lose the ticket
- **WHEN** the cancellation token fires after the LP registration exchange has begun
  (a ticket has been / is being spent)
- **THEN** the exchange still runs to completion (or its own error) and the resulting
  WireGuard configuration is returned rather than being discarded mid-spend

### Requirement: QUIC-bridge selection requires a non-empty identity pin

A dVPN directory entry SHALL only be treated as QUIC-bridge-capable if it carries a
non-empty `id_pubkey` identity pin. An entry with a blank pin MUST NOT be advertised
as QUIC-capable, so QUIC selection can never choose a bridge that lacks the pin its
certificate is verified against.

#### Scenario: Blank identity pin is not QUIC-capable
- **WHEN** a directory entry has an empty `id_pubkey`
- **THEN** it is not offered as a QUIC bridge and QUIC selection does not pick it

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

### Requirement: Gateway registration persistence and reuse

`nym-sdk-session` SHALL persist each successful gateway registration —
client WireGuard keypair, the gateway-returned WireGuard configuration
(assigned addresses, gateway public key, optional PSK, endpoint), gateway
identity, hop role, and registration time — in the session's data
directory, keyed by (network name, gateway identity, role). When a
registration is requested against a gateway and role for which a usable
cached entry exists (and reuse has not been disabled), the session SHALL
return a registration assembled from the cached state WITHOUT contacting
the gateway and WITHOUT spending a ticket. A fresh gateway exchange (and
its one-ticket spend) SHALL occur only for hops with no usable cached
entry — including partially cached two-hop requests, where only the missing
hop is registered. The session SHALL provide an invalidation operation
removing a cached entry by (gateway, role), enabling callers to fall back
to fresh registration when a cached peer no longer works; reuse SHALL be
default-on with a `SessionConfig` opt-out for callers requiring a fresh
peer per connection (the linkability trade-off of reuse SHALL be
documented on that option). The persisted file SHALL be written atomically,
created with owner-only permissions on unix, and treated as absent (never
a crash) when missing or unparseable. Entries older than a conservative
maximum age MAY be treated as absent and pruned.

#### Scenario: Repeat connection to a known gateway spends nothing

- **WHEN** a session registers with gateways it has previously registered
  with (same network, same roles) and the cached entries are usable
- **THEN** the returned registration is served from the cache, no LP
  exchange takes place, and no ticket is spent — verified by an unchanged
  `used_tickets` count

#### Scenario: Cache survives process restart

- **WHEN** a process registers, exits, and a new process creates a session
  over the same data directory
- **THEN** the new session finds and reuses the persisted registration

#### Scenario: Partial cache registers only the missing hop

- **WHEN** a two-hop registration finds a cached entry for one hop but not
  the other
- **THEN** only the uncached hop performs a gateway exchange and spends a
  ticket; the cached hop is reused

#### Scenario: Invalidation enables funds-safe fallback

- **WHEN** a caller invalidates a (gateway, role) entry after a cached
  registration failed to establish, and registers again
- **THEN** the entry is removed from the persistent cache and the new
  registration performs a fresh exchange (spending one ticket), which is
  then persisted in its place

#### Scenario: Opt-out forces fresh peers

- **WHEN** a session is configured with registration reuse disabled
- **THEN** every registration performs a fresh gateway exchange and neither
  reads nor is served from the cache

#### Scenario: Network isolation of cached entries

- **WHEN** the same data directory is used against a different network
- **THEN** cached entries recorded under another network name are never
  reused

#### Scenario: Corrupt or missing cache degrades to fresh registration

- **WHEN** the cache file is absent, unreadable, or fails to parse
- **THEN** the session behaves as if no registrations are cached (fresh
  exchange, then persist), and never fails or panics on the cache's account


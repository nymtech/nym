# dvpn-session Specification

## Purpose
Defines nym-sdk-session responsibilities: mnemonic-funded zk-nym ticketbook issuance, credential storage, gateway selection/registration, and QUIC-bridge entry selection.
## Requirements
### Requirement: Mnemonic-funded zk-nym ticketbook issuance

`nym-sdk-session` SHALL, from a caller-supplied mnemonic, deposit NYM on-chain and
obtain issued zk-nym ticketbooks from the Nym API signers, reusing
`nym-bandwidth-controller`/`nym-bandwidth-fetcher` **without modifying them**. It SHALL
support the wireguard ticket types `V1WireguardEntry` and `V1WireguardExit`, and it
MUST NOT deposit for any other ticket type (enforced by scoping the controller's
`managed_ticket_types`, and every explicit fetch, to the WireGuard types the session's
tunnel topology needs — both entry and exit for two-hop, entry only for single-hop, so
a single-hop session never provisions an unused exit ticketbook).

The session SHALL use the bandwidth controller in one of two modes, fixed when the
session is created:

- **Automatic top-up** (`SessionConfig::automatic_topups` set): the session SHALL
  construct the controller with the credential fetcher installed and run its event
  loop for the session's lifetime. The controller's own restock (startup sweep,
  periodic sweep, post-spend) provisions and replenishes the managed WireGuard types.
  `ensure_ticketbooks` SHALL wait for the required types to be stocked and spendable
  (`wait_for_ticketbooks`); if that reports the types as neither stocked nor being
  fetched, the session SHALL request a restock of exactly the required types once and
  wait again. The session MUST NOT install or uninstall the fetcher during
  provisioning. All ticket spending SHALL go through the running controller's request
  sender.
- **One-shot** (`automatic_topups` unset — the default): the session MUST NOT run a
  controller event loop. Ticket spending SHALL go through a non-running controller
  that has **no credential fetcher** (a read-only public-data fetcher only, so missing
  signing data can be fetched at spend time), so nothing on the spending path can
  deposit. `ensure_ticketbooks` SHALL create a short-lived, non-running controller
  with a freshly built credential fetcher and the topology-scoped managed types, read
  the stored stock, and for each required type that the controller's own restock
  predicate judges low or about to expire, issue one ticketbook via the controller's
  inline `fetch_ticketbook`; it SHALL then verify each required type is usable and
  tear the controller down: drop it and clean up its fetcher (closing the fetcher's
  pending-request recovery store). The provisioning controller SHALL share the
  session's credential-store handle rather than open a second one. A required type
  whose stock is sufficient MUST NOT be
  fetched. Each provisioning call SHALL build a fresh fetcher, and the on-disk
  recovery store SHALL carry any interrupted issuance forward across calls.

Provisioning SHALL be serialised so two concurrent `ensure_ticketbooks` calls cannot
issue twice for the same type. Provisioning MUST NOT depend on an empty
`managed_ticket_types` set combined with an explicit `restock_ticketbooks` request.
A caller already running a controller MAY supply its own `BandwidthTicketProvider`
instead, in which case the session spawns no controller and provisions nothing. The
client id used for issuance SHALL be derived from a hash of the mnemonic entropy
(never the raw entropy), and owned mnemonic material SHALL be zeroized on drop.

#### Scenario: Issue wireguard ticketbooks
- **WHEN** a caller provides a funded mnemonic and requests dVPN access
- **THEN** the session deposits NYM, contacts the signers, and obtains the WireGuard
  ticketbooks its topology needs (`V1WireguardEntry` and `V1WireguardExit` for two-hop,
  `V1WireguardEntry` only for single-hop)

#### Scenario: One-shot provisioning issues only what is missing
- **WHEN** a default (one-shot) session with no stored ticketbooks calls
  `ensure_ticketbooks` for two-hop
- **THEN** it issues one entry and one exit ticketbook via the inline fetch, both are
  stored and usable, and the short-lived controller is torn down

#### Scenario: One-shot provisioning skips a stocked type
- **WHEN** a default session holds a usable entry ticketbook above the restock
  threshold but its exit stock is exhausted, and `ensure_ticketbooks` is called for
  two-hop
- **THEN** exactly one exit ticketbook is issued and no entry deposit is made

#### Scenario: One-shot provisioning is a no-op when stocked
- **WHEN** a default session holds usable ticketbooks of every required type above the
  restock threshold
- **THEN** `ensure_ticketbooks` makes no deposit and returns success

#### Scenario: One-shot leaves nothing running
- **WHEN** a default session finishes `ensure_ticketbooks`, successfully or not
- **THEN** the provisioning controller has been dropped and its fetcher cleaned up, no
  controller event loop is running, and the session's spending provider has no
  credential fetcher — so no later deposit can occur

#### Scenario: Automatic top-up provisions from construction
- **WHEN** a session is created with automatic top-ups and no stored ticketbooks, and
  `ensure_ticketbooks` is called
- **THEN** the running controller's startup restock issues the managed WireGuard
  types and `ensure_ticketbooks` returns once they are stocked and spendable, without
  the session installing or uninstalling a fetcher

#### Scenario: Automatic top-up recovers from a failed startup fetch
- **WHEN** a session with automatic top-ups calls `ensure_ticketbooks` after the
  startup restock has failed, so the required types are neither stocked nor in flight
- **THEN** the session requests a restock of exactly the required types once and waits
  again, returning success if that fetch succeeds and the fetch error otherwise

#### Scenario: Setup phase is abortable without losing funds
- **WHEN** the caller cancels the provided cancellation/shutdown token during
  deposit/issuance
- **THEN** `ensure_ticketbooks` returns `Cancelled` promptly, no tunnel setup proceeds,
  the in-flight issuance is not dropped mid-way (it completes or is recorded in the
  pending-requests store), and any deposit already made is recovered on a later fetch

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
WireGuard role. Selection SHALL accept a set of **excluded** gateway identities that
are never returned. A pinned `Identity` spec SHALL never be silently substituted: if
the pinned identity is in the excluded set, selection fails with the
distinct-gateways error rather than returning a different gateway. `Country` and
`Random` specs SHALL skip every excluded identity when choosing among eligible
candidates; if excluding leaves no eligible candidate, selection fails with the
no-match error for that spec.

#### Scenario: Select by identity key
- **WHEN** the caller specifies a gateway identity key
- **THEN** the matching gateway is selected, or an error is returned if not found or
  it lacks WireGuard support

#### Scenario: Select by country code
- **WHEN** the caller specifies a two-letter country code
- **THEN** a WireGuard-capable gateway located in that country is chosen (randomly
  among matches, skipping any excluded identities), or an error is returned if none
  match

#### Scenario: Select randomly
- **WHEN** the caller requests a random gateway
- **THEN** a WireGuard-capable gateway is chosen uniformly at random from the eligible
  set

#### Scenario: Random selection skips an excluded set
- **WHEN** the caller requests a random gateway and passes a set of excluded
  identities
- **THEN** the chosen gateway is never one of the excluded identities; if every
  eligible gateway is excluded, selection fails with the no-gateway error

#### Scenario: Excluded pinned identity is not substituted
- **WHEN** the caller pins a gateway by identity that is also in the excluded set
- **THEN** selection fails with the distinct-gateways error and never returns a
  different gateway

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
horizon). Whether background restock happens SHALL be governed by the controller
mode: with the opt-in, the session runs the controller event loop with the credential
fetcher installed, and the controller's sweep restocks the managed WireGuard types per
policy; without the opt-in, no controller event loop runs and the spending provider has
no credential fetcher, so the session issues only what an explicit `ensure_ticketbooks`
call requires and never deposits in the background. In both modes the controller's
`managed_ticket_types` SHALL be the WireGuard ticket types the session's tunnel
topology needs (both entry and exit for two-hop, entry only for single-hop).

#### Scenario: Default session never restocks in the background
- **WHEN** a session is created without the automatic-topups opt-in and its stored
  tickets are gradually spent
- **THEN** no background deposit is ever made (no event loop runs and the spending
  provider cannot deposit); ticket exhaustion is observable so the caller can act

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

### Requirement: Two-hop registration can avoid implicated entry gateways

The session SHALL provide a two-hop registration path that accepts a set of **entry**
gateway identities to avoid, excluding them from entry selection so a caller that has
implicated an entry gateway (for example one that does not forward the tunnelled exit
handshake) can re-register onto a different entry. The exit selection SHALL continue
to exclude the chosen entry so the two hops stay distinct. When the entry spec is a
pinned identity that is in the avoid set, registration SHALL fail with the
distinct-gateways error rather than substituting a different gateway. The existing
two-hop registration entry point SHALL behave exactly as before, equivalent to
passing an empty avoid set. The QUIC two-hop registration SHALL offer the same
avoid-set variant, so a retrying caller can escape a non-forwarding entry behind a
QUIC bridge too.

Every two-hop registration entry point SHALL reject a session configured single-hop
(`two_hop = false`) up front with a dedicated topology-mismatch error — before any
topology fetch, ticketbook provisioning, or spend — since such a session manages
entry ticketbooks only and could never provision the exit ticketbook the registration
needs.

#### Scenario: QUIC two-hop registration honours the avoid set
- **WHEN** a QUIC two-hop registration is requested with a random entry spec and an
  avoid set containing a previously-implicated entry identity
- **THEN** the selected QUIC entry gateway is not in the avoid set

#### Scenario: Single-hop session rejects two-hop registration
- **WHEN** a session constructed with `two_hop = false` requests any two-hop
  registration
- **THEN** it fails immediately with the topology-mismatch error and spends nothing

#### Scenario: Random entry avoids an implicated entry
- **WHEN** a two-hop registration is requested with a random entry spec and an avoid
  set containing a previously-implicated entry identity
- **THEN** the selected entry gateway is not in the avoid set

#### Scenario: Pinned entry in the avoid set fails without substitution
- **WHEN** a two-hop registration is requested with a pinned entry identity that is
  in the avoid set
- **THEN** registration fails with the distinct-gateways error and no other gateway
  is substituted

#### Scenario: Empty avoid set preserves existing behavior
- **WHEN** a two-hop registration is requested with an empty avoid set
- **THEN** entry and exit are selected and registered exactly as the existing
  two-hop registration path does


## MODIFIED Requirements

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

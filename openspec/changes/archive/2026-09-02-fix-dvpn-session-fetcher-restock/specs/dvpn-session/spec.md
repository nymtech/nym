## MODIFIED Requirements

### Requirement: Mnemonic-funded zk-nym ticketbook issuance

`nym-sdk-session` SHALL, from a caller-supplied mnemonic, deposit NYM on-chain and
obtain issued zk-nym ticketbooks from the Nym API signers, reusing
`nym-bandwidth-controller`/`nym-bandwidth-fetcher`. It SHALL support the wireguard
ticket types `V1WireguardEntry` and `V1WireguardExit`, and it MUST NOT deposit for
any other ticket type (enforced by scoping the controller's `managed_ticket_types`
to the WireGuard types the session's tunnel topology needs — both entry and exit for
two-hop, entry only for single-hop, so a single-hop session never provisions an
unused exit ticketbook — and only ever requesting those). Once-off provisioning
(`ensure_ticketbooks`) SHALL drive issuance by the fetcher lifecycle: install a
credential fetcher (which triggers a restock of the managed WireGuard types), wait
until the required types are stocked and spendable, and then — unless automatic
top-ups are enabled — uninstall the fetcher so no further deposit occurs. Each
install SHALL build a FRESH fetcher: removing a fetcher (unset, or replace on retry)
`cleanup`s it, which for the `NyxdCredentialFetcher` closes its pending-request
recovery store, so the same instance cannot be reinstalled; the on-disk recovery
store carries any interrupted issuance forward across instances. Provisioning SHALL
be serialised so two concurrent `ensure_ticketbooks` calls cannot both install a
fetcher (the second replacing — and cleaning up — the first). Provisioning MUST NOT
depend on an empty `managed_ticket_types` set combined with an explicit
`restock_ticketbooks` request (which the controller evaluates against the managed set
and therefore skips when the set is empty). The session SHALL run the bandwidth
controller event loop and perform all ticket spending through its request sender (the
single-writer pattern); a caller already running a controller MAY supply its own
`BandwidthTicketProvider` instead, in which case the session spawns no controller.
The client id used for issuance SHALL be derived from a hash of the mnemonic entropy
(never the raw entropy), and owned mnemonic material SHALL be zeroized on drop.

#### Scenario: Issue wireguard ticketbooks
- **WHEN** a caller provides a funded mnemonic and requests dVPN access
- **THEN** the session deposits NYM, contacts the signers, and obtains
  `V1WireguardEntry`/`V1WireguardExit` ticketbooks

#### Scenario: Default-mode provisioning succeeds without an installed-at-construction fetcher
- **WHEN** a default session (no automatic top-ups) with no stored ticketbooks calls
  `ensure_ticketbooks`
- **THEN** it installs the credential fetcher to issue the required WireGuard
  ticketbooks, waits until they are stocked and spendable, and returns success — it
  does NOT fail with an "unavailable / none being fetched" error

#### Scenario: Provisioning removes the fetcher unless top-ups are enabled
- **WHEN** a default session finishes `ensure_ticketbooks`
- **THEN** the credential fetcher is uninstalled, so the controller's later proactive
  sweep has no fetcher and makes no background deposit

#### Scenario: Fetcher is removed even when provisioning fails
- **WHEN** a default session's `ensure_ticketbooks` fails (issuance error, readiness
  timeout, or cancellation) after the fetcher was installed
- **THEN** the credential fetcher is still uninstalled before the error is returned,
  so a failed provision never leaves background restock enabled

#### Scenario: Retrying a failed provision re-triggers a fetch
- **WHEN** a provision fails and is retried
- **THEN** the retry re-triggers issuance by installing a FRESH credential fetcher,
  rather than waiting on the periodic sweep — in default mode this happens on the next
  `ensure_ticketbooks` call (the fetcher was already removed on failure), and in
  automatic-top-up mode the session installs a fresh fetcher (the controller cleans up
  the previously-installed one)

#### Scenario: Each install builds a fresh fetcher
- **WHEN** provisioning installs a fetcher, removes it, and later installs again
- **THEN** a new fetcher instance is built for each install — a removed instance,
  whose recovery store was closed by `cleanup`, is never reinstalled

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

### Requirement: Opt-in automatic chain-side restock

Automatic on-chain ticketbook restocking SHALL be disabled by default. The session
SHALL offer an opt-in (e.g. `with_automatic_topups(policy)`) whose policy sets the
restock thresholds (minimum tickets, restock amount, check interval, soon-expiry
horizon). Whether background restock happens SHALL be governed by whether the
credential fetcher is left installed after provisioning, not by the managed set: the
controller's `managed_ticket_types` SHALL be the WireGuard ticket types the session's
tunnel topology needs (both entry and exit for two-hop, entry only for single-hop).
Enabling the opt-in SHALL leave the credential fetcher installed after
provisioning so the controller's sweep restocks per policy. Without the opt-in, the
session SHALL uninstall the fetcher after initial provisioning, so it issues only
what initial provisioning requires and never deposits in the background.

#### Scenario: Default session never restocks in the background
- **WHEN** a session is created without the automatic-topups opt-in and its stored
  tickets are gradually spent
- **THEN** no background deposit is ever made (the fetcher is not installed after
  provisioning); ticket exhaustion is observable so the caller can act

#### Scenario: Opted-in session restocks per policy
- **WHEN** automatic topups are enabled and the stored WireGuard tickets fall below
  the policy threshold
- **THEN** the controller deposits and issues new ticketbooks of the needed
  WireGuard type(s) without caller involvement

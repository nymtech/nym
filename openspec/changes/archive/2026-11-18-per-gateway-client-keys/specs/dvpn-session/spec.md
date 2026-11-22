## MODIFIED Requirements

### Requirement: Mnemonic-funded zk-nym ticketbook issuance

`nym-sdk-session` SHALL, from a caller-supplied mnemonic, deposit NYM on-chain and obtain issued zk-nym ticketbooks from the Nym API signers, reusing `nym-bandwidth-controller`/`nym-bandwidth-fetcher`. It SHALL support the wireguard ticket types `V1WireguardEntry` and `V1WireguardExit`, and it MUST NOT deposit for any other ticket type (enforced by scoping the controller's `managed_ticket_types` to the WireGuard types and only ever requesting those). The session SHALL run the bandwidth controller event loop and perform all ticket spending through its request sender (the single-writer pattern); a caller already running a controller MAY supply its own `BandwidthTicketProvider` instead, in which case the session spawns no controller.

The ecash user key used for issuance SHALL be derived from the mnemonic by the shared helper defined in the `ecash-user-key-derivation` capability, which hashes a purpose label together with the mnemonic entropy and never hands on the raw entropy. The session SHALL pass the mnemonic to the bandwidth fetcher and MUST NOT carry a derivation or a label of its own. Owned mnemonic material SHALL be zeroized on drop.

Moving to the shared helper changes the label, so a session issues new ticketbooks under a different ecash key than before this change. Ticketbooks and pending deposits it already holds remain usable, as required by `ecash-user-key-derivation`.

#### Scenario: Issue wireguard ticketbooks
- **WHEN** a caller provides a funded mnemonic and requests dVPN access
- **THEN** the session deposits NYM, contacts the signers, and obtains `V1WireguardEntry`/`V1WireguardExit` ticketbooks

#### Scenario: Setup phase is abortable without losing funds
- **WHEN** the caller cancels the provided cancellation/shutdown token during deposit/issuance
- **THEN** setup stops, no tunnel setup proceeds, and any deposit already made is recorded in the pending-requests store and recovered on a later fetch

#### Scenario: Mixnet ticket types are never purchased
- **WHEN** a session provisions, restocks, or tops up in any mode
- **THEN** no deposit is ever made for a non-WireGuard ticket type

#### Scenario: External provider preserves single writer
- **WHEN** the caller supplies an externally managed `BandwidthTicketProvider`
- **THEN** the session uses it for all spending and does not spawn its own controller over the credential store

#### Scenario: The session's ecash key matches other clients of the same mnemonic
- **WHEN** a session and an nym-sdk mixnet client buy ticketbooks with the same mnemonic
- **THEN** both issue under the same ecash public key

#### Scenario: Ticketbooks issued before the change are still spent
- **WHEN** a session holds ticketbooks issued under its previous, session-specific derivation and is upgraded
- **THEN** it spends tickets from those ticketbooks successfully

## ADDED Requirements

### Requirement: Free-tier capability token accepted in place of an ecash credential

The gateway SHALL accept a free-tier capability JWT as a `BandwidthCredential::FreeTier` variant presented at WireGuard registration, in place of a paid ecash (`ZkNym`) credential, and on success grant a free allowance. The token SHALL be a capability marker only; it MUST NOT be required to encode the allowance amount.

#### Scenario: Free token grants access

- **WHEN** a client registers presenting a valid free-tier JWT instead of an ecash credential
- **THEN** the gateway admits the peer and seeds a free allowance without any ecash verification

#### Scenario: Paid path unaffected

- **WHEN** a client registers with a `ZkNym` ecash credential
- **THEN** the existing paid verification and bandwidth-crediting path runs unchanged

### Requirement: Local JWT verification without nym-api or chain

The gateway SHALL verify the free-tier JWT locally using ed25519 against a configured attester public key, with no dependency on nym-api, the chain, or a JWKS endpoint, mirroring the existing upgrade-mode JWT verification.

#### Scenario: Valid signature accepted offline

- **WHEN** a free-tier JWT signed by the configured attester is verified
- **THEN** verification succeeds using only local cryptography and the configured public key

#### Scenario: Invalid or unknown-signer token rejected

- **WHEN** a token fails signature verification, is expired, or is signed by a key other than the configured attester
- **THEN** the gateway rejects the registration and grants no free allowance

### Requirement: No new ticket type and no nym-api impact

The free tier SHALL NOT introduce a new `TicketType` variant. The persisted client kind SHALL reuse an existing wireguard ticket type, and nym-api issuance SHALL be unaffected by this change.

#### Scenario: Peer persisted as an ordinary wireguard client

- **WHEN** a free-tier peer is registered
- **THEN** it is persisted with an existing wireguard client type and no new ticket type is required anywhere

### Requirement: Free allowance sourced from a network-wide constant

The free byte and time allowance SHALL be read from a `network-defaults` constant at redemption time rather than encoded in the token, so that a change to the constant applies uniformly and retroactively to already-issued tokens.

#### Scenario: Allowance change is retroactive

- **WHEN** the network-defaults free allowance constant is changed and a previously-issued free token is redeemed
- **THEN** the newly-configured allowance is applied

### Requirement: Free tier available on both registration transports

The free-tier credential SHALL be handled by the shared registration path so that both the LP transport and the legacy authenticator-over-mixnet transport accept it.

#### Scenario: Same credential works on either transport

- **WHEN** a client presents a free-tier JWT over the LP transport or over the authenticator-over-mixnet transport
- **THEN** both reach the shared `process_new_peer` path and are granted the free allowance

### Requirement: Client-side free-tier credential mirrors upgrade mode

The bandwidth controller SHALL represent the free-tier token as a `NymCredential::FreeTrialToken`, expose a `get_free_trial_token` provider method, and obtain it via a dedicated `FreeTrialFetcher`, mirroring the upgrade-mode token. The ecash-only `PreparedCredential` type MUST NOT be modified to carry the token.

#### Scenario: Token fetched externally and stored client-side

- **WHEN** the external client obtains a free-tier token from the VPN-API and injects it through the existing `CredentialFetcher`/store seam
- **THEN** it is persisted in a dedicated free-trial-token store (separate from the emergency-credential family) and later retrieved expiry-filtered, and `PreparedCredential` is unchanged

### Requirement: Token purpose distinguishes new-user trials from renewals

A free-tier capability token SHALL carry an explicit `purpose` claim, always set by the issuer, distinguishing a new-user trial from a subscription renewal. A new-user token grants the free allowance; a renewal token SHALL NOT grant free bandwidth and is instead confined immediately to the purchase walled garden. Until the walled garden exists, the gateway SHALL reject renewal tokens rather than grant free bandwidth.

#### Scenario: New-user token grants the free allowance

- **WHEN** a valid new-user free-tier token is presented at registration
- **THEN** the peer is granted the free allowance

#### Scenario: Renewal token is confined to purchase, never granted bandwidth

- **WHEN** a valid renewal free-tier token is presented at registration
- **THEN** no free bandwidth is granted; once the walled garden exists the peer is placed directly into it, and until then the registration is rejected

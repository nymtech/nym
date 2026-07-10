## ADDED Requirements

### Requirement: Mnemonic-funded zk-nym ticketbook issuance

`nym-sdk-session` SHALL, from a caller-supplied mnemonic, deposit NYM on-chain and
obtain issued zk-nym ticketbooks from the Nym API signers, reusing
`nym-bandwidth-controller`/`nym-bandwidth-fetcher`. It SHALL support the wireguard
ticket types `V1WireguardEntry` and `V1WireguardExit`.

#### Scenario: Issue wireguard ticketbooks
- **WHEN** a caller provides a funded mnemonic and requests dVPN access
- **THEN** the session deposits NYM, contacts the signers, and obtains
  `V1WireguardEntry`/`V1WireguardExit` ticketbooks

#### Scenario: Setup phase is abortable
- **WHEN** the caller cancels the provided `CancellationToken` during deposit/issuance
- **THEN** the issuance is aborted and no tunnel setup proceeds

### Requirement: Persistent credential storage for reuse

Issued ticketbooks SHALL be persisted in a credential store (`nym-credential-storage`)
so a subsequent tunnel bring-up reuses stocked tickets without re-depositing.

#### Scenario: Second bring-up reuses stored tickets
- **WHEN** a tunnel is brought down and later brought up again with the same store
- **THEN** the session uses already-stored ticketbooks and skips a new deposit if
  sufficient bandwidth remains

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

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

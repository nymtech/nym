## ADDED Requirements

### Requirement: WireGuard config export CLI (single-hop)

`smol-dvpn` SHALL provide a `smol-dvpn-config` example CLI that performs an LP
registration against a single gateway (`--gateway <spec>`) and prints a plain
WireGuard configuration usable with stock `wg`/`wg-quick`, including the client
private key and assigned address, and the peer's gateway public key, LP-negotiated
preshared key, endpoint, and allowed IPs.

#### Scenario: Emit a usable single-hop config
- **WHEN** the user runs `smol-dvpn-config --gateway <spec>` with a funded mnemonic
- **THEN** the CLI registers the gateway and prints a `[Interface]`/`[Peer]` config
  whose `PublicKey` and `PresharedKey` are the registration output

#### Scenario: Config is bandwidth-limited
- **WHEN** the exported config is used with stock WireGuard
- **THEN** it works only until the registered zk-nym bandwidth is exhausted, after
  which it disconnects (stock WireGuard performs no top-up)

### Requirement: Bandwidth top-up CLI

`smol-dvpn` SHALL provide a `smol-dvpn-topup` example CLI that spends a stored ticket
against the gateway `metadata` endpoint to extend the available bandwidth of an
existing registration.

#### Scenario: Top up an existing registration
- **WHEN** the user runs `smol-dvpn-topup` with a stored ticket available
- **THEN** the CLI submits the ticket to the gateway metadata endpoint and reports the
  updated available bandwidth

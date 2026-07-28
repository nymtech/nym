# dvpn-tools Specification

## Purpose
Defines the nym-smoldvpn example CLIs and tools: config export, bandwidth top-up, gRPC-through-tunnel, public-IP relocation, and Zcash sync benchmarks.
## Requirements
### Requirement: WireGuard config export CLI (single-hop)

`nym-smoldvpn` SHALL provide a `smoldvpn-config` example CLI that performs an LP
registration against a single gateway (`--gateway <spec>`) and prints a plain
WireGuard configuration usable with stock `wg`/`wg-quick`, including the client
private key and assigned address, and the peer's gateway public key, LP-negotiated
preshared key, endpoint, and allowed IPs.

#### Scenario: Emit a usable single-hop config
- **WHEN** the user runs `smoldvpn-config --gateway <spec>` with a funded mnemonic
- **THEN** the CLI registers the gateway and prints a `[Interface]`/`[Peer]` config
  whose `PublicKey` and `PresharedKey` are the registration output

#### Scenario: Config is bandwidth-limited
- **WHEN** the exported config is used with stock WireGuard
- **THEN** it works only until the registered zk-nym bandwidth is exhausted, after
  which it disconnects (stock WireGuard performs no top-up)

### Requirement: Bandwidth top-up CLI

`nym-smoldvpn` SHALL provide a `smoldvpn-topup` example CLI that spends a stored ticket
against the gateway `metadata` endpoint to extend the available bandwidth of an
existing registration.

#### Scenario: Top up an existing registration
- **WHEN** the user runs `smoldvpn-topup` with a stored ticket available
- **THEN** the CLI submits the ticket to the gateway metadata endpoint and reports the
  updated available bandwidth

### Requirement: gRPC-through-tunnel example

`nym-smoldvpn` SHALL provide a `smoldvpn-grpc` example that brings up a tunnel and
issues a real `tonic` gRPC request through it via the tunnel connector.

#### Scenario: gRPC request over the tunnel
- **WHEN** the user runs `smoldvpn-grpc` with a funded mnemonic
- **THEN** the example brings up a tunnel and completes a gRPC call through the tunnel
  connector

### Requirement: Public-IP relocation examples

`nym-smoldvpn` SHALL provide `two-hop-ip` and `two-hop-quic` examples that query a public
IP-echo service directly and then through the tunnel, demonstrating that the observed
public IP/location becomes the exit gateway's. `two-hop-quic` SHALL front the entry
leg with a QUIC bridge (selecting a QUIC-capable entry gateway).

#### Scenario: IP relocates through the tunnel
- **WHEN** the user runs `two-hop-ip` with a funded mnemonic
- **THEN** the pre-tunnel public IP differs from the through-tunnel IP, and the latter
  corresponds to the exit gateway

#### Scenario: QUIC-fronted entry
- **WHEN** the user runs `two-hop-quic`
- **THEN** the entry gateway is QUIC-bridge-capable and the entry leg is carried over
  the QUIC bridge

### Requirement: Zcash compact-block sync benchmark example

`nym-smoldvpn` SHALL provide a `zcash-sync` example that syncs a configurable number of
Zcash compact blocks (default 10,000, selectable with `--blocks <N>`) from a public
`lightwalletd` over gRPC-over-TLS both directly and through the tunnel, and reports the
throughput of each.

#### Scenario: Sync direct vs. through the tunnel
- **WHEN** the user runs `zcash-sync` with a funded mnemonic
- **THEN** the example reports blocks synced and elapsed time for both the direct and
  the through-tunnel path

### Requirement: Shared example selection CLI

The configurable examples (`two-hop-ip`, `two-hop-quic`, `zcash-sync`) SHALL share a
command-line interface for choosing hop mode (`--one-hop`/`--two-hop`), gateways
(`--entry`/`--exit`/`--gateway <spec>`, where `<spec>` is `random`, a two-letter
country code, or a base58 identity), QUIC entry (`--quic`), and — for `zcash-sync` —
the block count (`--blocks <N>`), rejecting invalid combinations (QUIC requires
two-hop).

#### Scenario: Select gateways and mode from the CLI
- **WHEN** the user passes `--entry`/`--exit`/`--gateway`/`--one-hop`/`--two-hop`
- **THEN** the example provisions the corresponding hop mode and gateway selection

#### Scenario: Reject QUIC with single-hop
- **WHEN** the user passes `--quic --one-hop`
- **THEN** the example exits with an error explaining QUIC is two-hop-entry only


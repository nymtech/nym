# dvpn-quic-bridge Specification

## Purpose
Defines the WgPacketTransport data-plane abstraction and the QUIC bridge client used to front the two-hop entry gateway leg in nym-smoldvpn.
## Requirements
### Requirement: WgPacketTransport abstraction with three data-plane modes

The datapath SHALL send and receive one WireGuard packet per operation through a
`WgPacketTransport` seam, supporting exactly three data-plane modes: one-hop,
two-hop, and QUIC-tunnelling two-hop. QUIC bridging SHALL apply only to the two-hop
entry-gateway leg (the bridge is bound 1:1 to a gateway); there is no QUIC one-hop
mode.

#### Scenario: Direct transport uses UDP
- **WHEN** the `Direct` transport is selected
- **THEN** each WireGuard packet is sent as a real UDP datagram to the entry gateway
  endpoint

#### Scenario: QUIC bridge only on two-hop
- **WHEN** the caller selects QUIC bridging
- **THEN** it is applied to the two-hop entry leg, and configuring QUIC for a one-hop
  tunnel is rejected

### Requirement: QUIC bridge client reimplemented inline

The QUIC bridge client SHALL be implemented inline in `nym-smoldvpn` using `quinn`
declared in the crate's own `Cargo.toml`, and SHALL NOT depend on the `nym_bridges`
crate. It SHALL byte-match the bridge protocol: ALPN `hq-29`; ed25519-based server
certificate pinning (SNI/CN ∈ alt-names and certificate SPKI equal to the pinned
identity public key, ED25519-only verify schemes); and one reliable QUIC
bidirectional stream carrying WireGuard packets each prefixed by a 2-byte big-endian
length.

#### Scenario: Length-framed WireGuard packets over one bi-stream
- **WHEN** the client sends a WireGuard packet over the bridge
- **THEN** it writes a 2-byte big-endian length followed by the packet on a single
  QUIC bidirectional stream, and reads inbound packets by the same framing

#### Scenario: Server identity pinning enforced
- **WHEN** the bridge presents a certificate whose SPKI does not equal the pinned
  ed25519 `id_pubkey`, or whose SNI/CN is not an accepted alt-name
- **THEN** the connection is rejected

#### Scenario: No dependency on nym_bridges crate
- **WHEN** the crate is built
- **THEN** `nym-smoldvpn` does not depend on the `nym_bridges` crate

### Requirement: Bridge parameters from the gateway directory

The client SHALL obtain bridge connection parameters — bridge addresses, SNI host,
and the base64 ed25519 identity public key — from the gateway directory / VPN API per
gateway, rather than hand-specifying them, and SHALL send no client-side
gateway-selection handshake to the bridge.

#### Scenario: Params sourced per gateway
- **WHEN** a gateway with a configured QUIC bridge is selected
- **THEN** its bridge addresses, SNI host, and id_pubkey are taken from the directory
  and used to connect

#### Scenario: No gateway-selection handshake
- **WHEN** the client connects to the bridge
- **THEN** it opens the bi-stream and forwards WireGuard packets without sending any
  target-gateway control message

### Requirement: Long-lived session tuning

The QUIC client SHALL configure a keep-alive interval, a max idle timeout, and the BBR
congestion controller so the long-lived tunnel session does not drop on idle. The
connect operation SHALL be cancellable via the tunnel's `CancellationToken`.

#### Scenario: Idle session stays open
- **WHEN** the tunnel is idle for a period shorter than the max idle timeout
- **THEN** the QUIC session remains open due to keep-alive

#### Scenario: Connect is cancellable
- **WHEN** the token is cancelled while the QUIC connection is being established
- **THEN** the connect attempt is aborted

### Requirement: Registration remains direct

LP registration SHALL run directly (TCP to the gateway LP port) and SHALL NOT be
carried over the QUIC bridge. Only the WireGuard data plane may use the bridge.

#### Scenario: Register direct, then bridge the data plane
- **WHEN** a tunnel is set up with QUIC bridging enabled
- **THEN** LP registration is performed directly first, and only the subsequent
  WireGuard data plane uses the QUIC bridge


# Delta: dvpn-quic-bridge — crate renamed to `nym-smoldvpn`

## MODIFIED Requirements

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

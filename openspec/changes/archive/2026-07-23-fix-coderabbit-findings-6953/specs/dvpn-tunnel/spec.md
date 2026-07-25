# dvpn-tunnel Specification (delta)

## ADDED Requirements

### Requirement: QUIC bridge connect is fully bounded by timeout and cancellation

Establishing the QUIC bridge transport SHALL apply the connect timeout and the
caller's cancellation token to the entire connect sequence — both the QUIC handshake
and the opening of the WireGuard-carrying bi-stream — so a stalled or adversarial
bridge cannot leave `connect` waiting indefinitely.

#### Scenario: Stalled bi-stream open is bounded
- **WHEN** the QUIC handshake succeeds but the bridge never grants the bi-stream
  (e.g. withholds stream credit)
- **THEN** the connect attempt fails on timeout or cancellation rather than hanging

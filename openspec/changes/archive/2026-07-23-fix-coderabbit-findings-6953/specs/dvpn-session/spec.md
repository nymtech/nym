# dvpn-session Specification (delta)

## ADDED Requirements

### Requirement: Registration is not cancellable once a ticket is spent

Gateway registration SHALL check cancellation before starting the LP registration
exchange, but once the exchange (which spends a WireGuard ticket) has begun it MUST
run to completion or its own failure without being aborted by the caller's
cancellation token — so a cancel cannot drop the future after the gateway has
processed the ticket spend, which would lose the ticket with no configuration
returned. Gateway selection (before any spend) SHALL remain promptly cancellable.

#### Scenario: Cancel before the exchange aborts cleanly
- **WHEN** the cancellation token fires before registration begins the LP exchange
- **THEN** registration returns a cancelled error and no ticket is spent

#### Scenario: Cancel during the exchange does not lose the ticket
- **WHEN** the cancellation token fires after the LP registration exchange has begun
  (a ticket has been / is being spent)
- **THEN** the exchange still runs to completion (or its own error) and the resulting
  WireGuard configuration is returned rather than being discarded mid-spend

### Requirement: QUIC-bridge selection requires a non-empty identity pin

A dVPN directory entry SHALL only be treated as QUIC-bridge-capable if it carries a
non-empty `id_pubkey` identity pin. An entry with a blank pin MUST NOT be advertised
as QUIC-capable, so QUIC selection can never choose a bridge that lacks the pin its
certificate is verified against.

#### Scenario: Blank identity pin is not QUIC-capable
- **WHEN** a directory entry has an empty `id_pubkey`
- **THEN** it is not offered as a QUIC bridge and QUIC selection does not pick it

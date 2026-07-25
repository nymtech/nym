# Delta: dvpn-tunnel — awaitable session establishment

## ADDED Requirements

### Requirement: Awaitable per-hop session establishment

The tunnel SHALL expose an awaitable operation that resolves once the
WireGuard session(s) required by the tunnel shape are established (the
entry hop, and additionally the exit hop for two-hop tunnels), bounded by a
caller-supplied timeout. On timeout the result SHALL report per-hop
establishment status (which hop(s) failed to establish), so callers can act
on exactly the failed hop — e.g. invalidating one cached registration
rather than both. The signal SHALL be driven by the datapath's own
handshake-progress tracking (the same markers logged at info), not by
probing application traffic, and SHALL remain correct across transports
(direct UDP and QUIC bridge).

#### Scenario: Healthy bring-up resolves promptly

- **WHEN** a tunnel is built against reachable gateways and the caller
  awaits establishment with a generous bound
- **THEN** the await resolves successfully once all required hops have
  completed their handshakes (observed healthy two-hop: well under a
  second)

#### Scenario: Dead cached registration is detected within the bound

- **WHEN** a tunnel is built from stale registration state whose gateway
  peer no longer exists, and the caller awaits establishment
- **THEN** the await fails at the timeout with the non-established hop(s)
  identified, enabling the invalidate-and-re-register fallback

#### Scenario: Exit-only failure is attributed to the exit hop

- **WHEN** the entry hop establishes but the exit hop does not within the
  bound
- **THEN** the failure reports entry established and exit not established

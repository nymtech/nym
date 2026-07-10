# dvpn-tunnel Specification (delta)

## MODIFIED Requirements

### Requirement: Bandwidth top-up for a long-lived tunnel

While connected, the tunnel SHALL top up bandwidth by spending stored tickets via
the gateway `metadata` endpoint before the registered bandwidth is exhausted.
Gateway-side top-up SHALL be enabled by default for tunnels built from a session
registration (the credential source is supplied automatically); callers MAY
disable or tune it. All metadata-endpoint traffic (bandwidth queries and top-up
spends) MUST travel through the tunnel itself, never the host network.

#### Scenario: Top-up before exhaustion
- **WHEN** remaining bandwidth on a running tunnel falls low and stored tickets exist
- **THEN** a ticket is spent against the gateway metadata endpoint and available
  bandwidth increases

#### Scenario: Default-on for session-built tunnels
- **WHEN** a tunnel is built from a session registration without top-up
  configuration
- **THEN** gateway-side top-up is active using the session's ticket provider

#### Scenario: Metadata traffic stays in-tunnel
- **WHEN** the tunnel queries available bandwidth or spends a top-up ticket
- **THEN** the HTTP request is carried through the tunnel datapath and the
  client's real IP is never exposed to the metadata endpoint

### Requirement: Configurable and dynamically adjustable MTU

The tunnel MTU SHALL be configurable and changeable at runtime while the tunnel is up,
defaulting to the reference values (overhead 80 B/hop; desktop entry 1420 / exit 1340;
mobile entry 1360 / exit 1280). On iOS and Android targets the mobile defaults SHALL
apply automatically. A runtime MTU change MUST NOT terminate the datapath, and
concurrent MTU-change calls MUST NOT leave the published stack and the datapath's
channels referring to different stacks.

#### Scenario: Default MTU applied
- **WHEN** a tunnel is connected without an MTU override
- **THEN** the platform-appropriate default MTU is applied and per-hop overhead is
  subtracted correctly

#### Scenario: MTU changed at runtime
- **WHEN** the caller sets a new MTU on a running tunnel
- **THEN** the smoltcp interface is resized and per-hop MTUs re-derived without
  tearing the tunnel down

#### Scenario: Datapath survives the stack swap
- **WHEN** the old stack is dropped as part of a runtime MTU change
- **THEN** the datapath observes the channel swap (not the old channel's closure)
  and continues passing traffic

## ADDED Requirements

### Requirement: Bandwidth event stream

The tunnel SHALL expose a subscribable bandwidth event stream reporting at least:
bandwidth low (with available amount and threshold), top-up succeeded (with new
available amount), top-up failed (with reason), and bandwidth exhausted, plus a
latest-reading accessor. Monitoring SHALL run whenever a metadata endpoint is
known, independently of whether automatic top-up is enabled, so implementers can
prompt their users to obtain ticketbooks manually.

#### Scenario: Implementer notified on low bandwidth
- **WHEN** available bandwidth falls below the configured threshold on a tunnel
  with automatic top-up disabled
- **THEN** a low-bandwidth event is delivered to subscribers

#### Scenario: Top-up outcomes are observable
- **WHEN** an automatic top-up succeeds or fails
- **THEN** a corresponding event (with new balance or failure reason) is delivered

### Requirement: Transport failure handling

The datapath SHALL classify transport receive errors: fatal errors (e.g. a closed
QUIC bridge stream) SHALL terminate the datapath observably; transient errors
SHALL be logged with backoff. The datapath MUST NOT spin retrying a permanently
failed transport.

#### Scenario: Closed bridge terminates the datapath
- **WHEN** the QUIC bridge stream closes while the tunnel is running
- **THEN** the datapath exits (observable to the caller) instead of logging the
  same error in a tight loop

### Requirement: IP-literal destinations bypass DNS

Tunnel connect-by-host surfaces (including the tower connector) SHALL detect
IP-literal hosts (including bracketed IPv6 forms from URIs) and connect directly
without a DNS query.

#### Scenario: Connector dials a raw-IP URI
- **WHEN** a connector is asked to reach `http://10.0.0.1:50051`
- **THEN** the TCP connection is opened to `10.0.0.1:50051` directly with no DNS
  lookup

### Requirement: IPv4 bridge address preference

The tunnel SHALL dial an IPv4 bridge address when bridge parameters list
multiple addresses and the client transport is IPv4-only (the directory lists
IPv6 first, and the bridge client dials only the first address given).

#### Scenario: v4-only client connects to a dual-listed bridge
- **WHEN** bridge params list an IPv6 address before an IPv4 address
- **THEN** the QUIC bridge connection is attempted against the IPv4 address

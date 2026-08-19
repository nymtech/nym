# dvpn-tunnel Specification

## Purpose
Defines the nym-smoldvpn userspace WireGuard tunnel: boringtun datapath, single/two-hop modes, tokio traffic surfaces, lifecycle, DNS, MTU, and bandwidth top-up.
## Requirements
### Requirement: Userspace WireGuard datapath with boringtun

`nym-smoldvpn` SHALL implement the WireGuard datapath using `boringtun` in userspace,
with no OS `tun` device and no root. It MUST NOT use `defguard_wireguard_rs`,
`wireguard-go`, or any Go/FFI engine. Each WireGuard peer's public key and preshared
key SHALL be taken from the session's registration output.

#### Scenario: Bring up a tunnel with no OS interface
- **WHEN** a tunnel is connected with valid per-hop WireGuard configuration
- **THEN** encryption/decryption occurs in-process via `boringtun` and no OS network
  interface is created and no elevated privileges are required

#### Scenario: Peer configured from registration
- **WHEN** a WireGuard peer is configured
- **THEN** its public key and preshared key are those returned by gateway
  registration

### Requirement: Single-hop and two-hop modes

The tunnel SHALL support a single-hop mode selected by one gateway (`gateway=…`) and a
two-hop mode selected by distinct entry and exit gateways (`entry=…`/`exit=…`).

#### Scenario: Single-hop tunnel
- **WHEN** the caller selects single-hop with one gateway
- **THEN** a single `boringtun` tunnel to that gateway carries application traffic with
  no inner encapsulation

#### Scenario: Two-hop nested tunnel
- **WHEN** the caller selects two-hop with entry and exit gateways
- **THEN** application packets are encapsulated by the exit tunnel, framed as IP/UDP to
  the exit endpoint, and re-encapsulated by the entry tunnel to the entry gateway

### Requirement: tokio traffic surfaces

The tunnel SHALL expose `tcp_connect` (`AsyncRead+AsyncWrite`), UDP sockets, and
connector adapters usable by `tonic`, `hyper`, and `reqwest`, so application traffic —
including `tonic` gRPC — flows inside the tunnel.

#### Scenario: gRPC over the tunnel
- **WHEN** a `tonic` client is built with the tunnel's connector and issues a request
- **THEN** the gRPC traffic is carried through the tunnel to the destination

#### Scenario: Raw TCP and UDP
- **WHEN** the caller uses `tcp_connect` or a UDP socket from the tunnel
- **THEN** the bytes/datagrams traverse the tunnel

### Requirement: Cancellation-driven lifecycle

The tunnel SHALL accept a `CancellationToken` that aborts the setup phase before the
tunnel is up and tears down the long-lived tunnel once connected; an explicit
`shutdown()` SHALL have the same teardown effect. Issued tickets SHALL remain in the
store after teardown.

#### Scenario: Abort during setup
- **WHEN** the token is cancelled during provisioning/registration
- **THEN** setup stops and no tunnel is established

#### Scenario: Teardown while connected
- **WHEN** the token is cancelled (or `shutdown()` is called) on a running tunnel
- **THEN** background tasks stop, the tunnel closes, and stored tickets are retained

### Requirement: DNS in-tunnel by default

The tunnel SHALL resolve DNS through the tunnel by default and allow this to be
configured (disabled or pointed at a specific resolver).

#### Scenario: Default in-tunnel resolution
- **WHEN** the caller resolves a hostname via the tunnel with default settings
- **THEN** the DNS query is sent through the tunnel, not the host resolver

#### Scenario: DNS configuration honored
- **WHEN** the caller disables in-tunnel DNS or sets a specific resolver
- **THEN** resolution follows the configured behavior

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

### Requirement: boringtun timer maintenance

The tunnel SHALL drive `boringtun` timers from a dedicated cancellable background task,
routing timer-generated output (keepalive, handshake, rekey) through the active
transport.

#### Scenario: Timer output is transmitted
- **WHEN** boringtun emits timer-driven packets (e.g. a keepalive or rekey)
- **THEN** they are sent through the active `WgPacketTransport`

#### Scenario: Timer task stops on teardown
- **WHEN** the tunnel is torn down
- **THEN** the timer task is cancelled

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

### Requirement: QUIC bridge connect is fully bounded by timeout and cancellation

Establishing the QUIC bridge transport SHALL apply the connect timeout and the
caller's cancellation token to the entire connect sequence — both the QUIC handshake
and the opening of the WireGuard-carrying bi-stream — so a stalled or adversarial
bridge cannot leave `connect` waiting indefinitely.

#### Scenario: Stalled bi-stream open is bounded
- **WHEN** the QUIC handshake succeeds but the bridge never grants the bi-stream
  (e.g. withholds stream credit)
- **THEN** the connect attempt fails on timeout or cancellation rather than hanging

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


## ADDED Requirements

### Requirement: Userspace WireGuard datapath with boringtun

`nym-smol-dvpn` SHALL implement the WireGuard datapath using `boringtun` in userspace,
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
mobile entry 1360 / exit 1280).

#### Scenario: Default MTU applied
- **WHEN** a tunnel is connected without an MTU override
- **THEN** the platform-appropriate default MTU is applied and per-hop overhead is
  subtracted correctly

#### Scenario: MTU changed at runtime
- **WHEN** the caller sets a new MTU on a running tunnel
- **THEN** the smoltcp interface is resized and per-hop MTUs re-derived without
  tearing the tunnel down

### Requirement: Bandwidth top-up for a long-lived tunnel

While connected, the tunnel SHALL top up bandwidth by spending stored tickets via the
gateway `metadata` endpoint before the registered bandwidth is exhausted.

#### Scenario: Top-up before exhaustion
- **WHEN** remaining bandwidth on a running tunnel falls low and stored tickets exist
- **THEN** a ticket is spent against the gateway metadata endpoint and available
  bandwidth increases

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

## ADDED Requirements

### Requirement: Transport-agnostic userspace TCP/IP stack

`smol-core` SHALL provide a pure-Rust, `tokio`-async userspace TCP/IP stack driven by
a caller-supplied bidirectional transport of raw IP packets (`Vec<u8>`), with no OS
`tun` device and no elevated privileges. It MUST NOT depend on Go, a gVisor netstack,
or any FFI network stack.

#### Scenario: Stack driven by an abstract IP-packet transport
- **WHEN** a caller constructs the stack with a transport that yields inbound IP
  packets and accepts outbound IP packets
- **THEN** the stack routes application socket traffic to/from that transport without
  requiring any OS network interface or root privileges

#### Scenario: Assigned tunnel address configures the interface
- **WHEN** the stack is built with the tunnel's assigned IPv4/IPv6 address
- **THEN** the smoltcp interface is configured with those addresses and outbound
  packets carry them as source

### Requirement: TCP socket surface

The stack SHALL expose a `tcp_connect(addr)` API returning a stream that implements
`tokio::io::AsyncRead + AsyncWrite`, usable as a drop-in for `tokio::net::TcpStream`.

#### Scenario: TCP connect through the tunnel
- **WHEN** the caller calls `tcp_connect(addr)` and the handshake succeeds
- **THEN** a stream implementing `AsyncRead + AsyncWrite` is returned and its
  read/write bytes traverse the transport

#### Scenario: Connection failure surfaces an error
- **WHEN** the TCP handshake fails (refused/timeout) or the stack has shut down
- **THEN** `tcp_connect` returns an I/O error rather than a stream

### Requirement: UDP socket surface

The stack SHALL expose UDP sockets bound to an ephemeral or specified port that send
and receive datagrams through the transport.

#### Scenario: UDP datagram round-trip
- **WHEN** the caller obtains a UDP socket and sends a datagram to a reachable
  destination
- **THEN** the datagram is emitted through the transport and replies are delivered to
  the socket

### Requirement: Tunnel-scoped DNS resolver

The stack SHALL provide a DNS resolver that performs lookups over a stack UDP socket,
so name resolution occurs through the transport rather than the host resolver.

#### Scenario: In-stack name resolution
- **WHEN** a hostname is resolved via the stack's resolver
- **THEN** the DNS query is sent over a stack UDP socket and not the host's system
  resolver

### Requirement: smolmix refactored onto smol-core

The existing `smolmix` crate SHALL be refactored to consume `smol-core` for its
stack, while preserving its existing public API.

#### Scenario: smolmix public API unchanged
- **WHEN** `smolmix` is rebuilt on top of `smol-core`
- **THEN** existing `smolmix` public items (e.g. `Tunnel`, `tcp_connect`,
  `udp_socket`) continue to compile and behave as before

# smol-core-stack Specification

## Purpose
Defines nym-smol-core, the transport-agnostic userspace TCP/IP stack, its TCP/UDP/DNS socket surfaces, and the smolmix refactor onto it.
## Requirements
### Requirement: Transport-agnostic userspace TCP/IP stack

`nym-smol-core` SHALL provide a pure-Rust, `tokio`-async userspace TCP/IP stack driven by
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

The stack SHALL provide a DNS resolver that performs lookups over a stack UDP
socket, so name resolution occurs through the transport rather than the host
resolver. Each query SHALL use a randomly generated transaction id, and a
response SHALL be accepted only if its id matches the query and its source is
the configured DNS server; non-matching datagrams SHALL be discarded and reading
SHALL continue until a match or the query timeout. Server failure response codes
(e.g. SERVFAIL, REFUSED) SHALL surface as errors distinct from NXDOMAIN/empty
results. While the stack interface is IPv4-only, the resolver SHALL NOT return
IPv6 addresses (the AAAA query is skipped).

#### Scenario: In-stack name resolution
- **WHEN** a hostname is resolved via the stack's resolver
- **THEN** the DNS query is sent over a stack UDP socket and not the host's system
  resolver

#### Scenario: Mismatched response is discarded, lookup still succeeds
- **WHEN** a datagram with a non-matching transaction id or source arrives before
  the genuine reply
- **THEN** it is discarded and the genuine reply is still accepted within the
  timeout

#### Scenario: Server failure distinct from no-records
- **WHEN** the upstream server answers SERVFAIL or REFUSED
- **THEN** the resolver returns a server-failure error, not a no-records error

#### Scenario: No unroutable IPv6 results on a v4-only stack
- **WHEN** a name with both A and AAAA records is resolved on an IPv4-only stack
- **THEN** only IPv4 addresses are returned

### Requirement: smolmix refactored onto smol-core

The existing `smolmix` crate SHALL be refactored to consume `nym-smol-core` for its
stack, while preserving its existing public API.

#### Scenario: smolmix public API unchanged
- **WHEN** `smolmix` is rebuilt on top of `nym-smol-core`
- **THEN** existing `smolmix` public items (e.g. `Tunnel`, `tcp_connect`,
  `udp_socket`) continue to compile and behave as before

### Requirement: IP-literal hosts bypass resolution

`tcp_connect_host` (and the resolver path behind it) SHALL detect a host string
that parses as an IP address and connect to it directly without issuing any DNS
query.

#### Scenario: Connect to an IPv4 literal
- **WHEN** `tcp_connect_host("10.0.0.1", 50051)` is called
- **THEN** the TCP connection is opened to `10.0.0.1:50051` with no DNS query
  emitted

### Requirement: Truncated DNS responses are rejected

The DNS resolver SHALL reject a response whose truncation (`TC`) bit is set rather
than returning the partial set of answers it happens to carry. A truncated response
is incomplete by definition (RFC 1035 requires retrying over TCP), so it MUST surface
as an error instead of being treated as an authoritative answer.

#### Scenario: Truncated response does not yield partial addresses
- **WHEN** the upstream returns a response with the `TC` bit set
- **THEN** the resolver returns an error rather than the partial answers in that
  response


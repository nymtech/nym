# smol-core-stack Specification (delta)

## MODIFIED Requirements

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

## ADDED Requirements

### Requirement: IP-literal hosts bypass resolution

`tcp_connect_host` (and the resolver path behind it) SHALL detect a host string
that parses as an IP address and connect to it directly without issuing any DNS
query.

#### Scenario: Connect to an IPv4 literal
- **WHEN** `tcp_connect_host("10.0.0.1", 50051)` is called
- **THEN** the TCP connection is opened to `10.0.0.1:50051` with no DNS query
  emitted

# smol-core-stack Spec Delta

## REMOVED Requirements

### Requirement: Tunnel-scoped DNS resolver

**Reason**: The transport changes from plain UDP:53 to DoH, which removes the
UDP-specific off-path datagram concern (source-address and transaction-id matching,
truncation-bit retry). Replaced by "Tunnel-scoped DNS resolver over DoH" below, which
keeps the transport-independent guarantees (in-stack resolution, SERVFAIL distinct from
NXDOMAIN, IPv4-only).

**Migration**: Callers are unaffected; `resolve(tunnel, hostname)` keeps its signature.
Only the transport behind it changes.

## ADDED Requirements

### Requirement: Tunnel-scoped DNS resolver over DoH

The stack SHALL provide a DNS resolver that performs lookups over DNS-over-HTTPS
(DoH, RFC 8484) through the tunnel, so name resolution occurs through the transport
rather than the host resolver and is encrypted end to end to the resolver. Each query
SHALL be sent as an HTTP GET with the base64url (no padding) wire-format message in the
`dns` query parameter and `accept: application/dns-message`, to the resolver's
`/dns-query` endpoint, and the response body SHALL be parsed as a wire-format message.
The GET form is chosen because it is idempotent, so it reuses the client's existing
idempotent-retry and connection-pooling path. The DoH request SHALL use the tunnel's
existing TLS and HTTP client stack, not a DNS-library DoH transport. Because the
resolver is a DoH-only path, the DNS resolver SHALL depend on that TLS and HTTP stack
being compiled in; a build that exposes the standalone DNS entry point SHALL pull the
HTTP stack. Server failure
response codes (e.g. SERVFAIL, REFUSED) SHALL surface as errors distinct from
NXDOMAIN/empty results. While the stack interface is IPv4-only, the resolver SHALL NOT
return IPv6 addresses (the AAAA query is skipped). Because the response is the body of
an HTTP reply over an authenticated TLS channel, the off-path datagram checks that a
UDP resolver needs (source-address and transaction-id matching) do not apply and SHALL
NOT be required on the DoH path.

#### Scenario: In-stack name resolution over DoH

- **WHEN** a hostname is resolved via the stack's resolver
- **THEN** the query is sent as a DoH POST over the tunnel's TLS/HTTP stack and not the
  host's system resolver

#### Scenario: Server failure distinct from no-records

- **WHEN** the upstream resolver answers SERVFAIL or REFUSED
- **THEN** the resolver returns a server-failure error, not a no-records error

#### Scenario: No unroutable IPv6 results on a v4-only stack

- **WHEN** a name with both A and AAAA records is resolved on an IPv4-only stack
- **THEN** only IPv4 addresses are returned

### Requirement: DNS rate-limiting and server errors are visible

The resolver SHALL map an HTTP `429` response from a DoH endpoint to a distinct
rate-limited error, and other non-success HTTP statuses to a distinct server-error
value carrying the status. Both SHALL be logged unconditionally (not only under a debug
build), so rate-limiting is visible in production. A rate-limited resolver SHALL be
reported as such rather than producing a silent stall until a timeout expires, and when
every endpoint is rate-limited the rate-limited error SHALL be the one surfaced to the
caller, not a generic timeout.

#### Scenario: Rate-limited resolver is reported

- **WHEN** a DoH endpoint answers `429 Too Many Requests`
- **THEN** the resolver returns a rate-limited error naming the endpoint, rather than
  waiting out the attempt timeout with no diagnosis

#### Scenario: Server error carries the status

- **WHEN** a DoH endpoint answers a non-success status other than 429
- **THEN** the resolver returns a server-error value carrying that status

### Requirement: Fast per-attempt DNS timeout with backup endpoint

Each DoH attempt SHALL be bounded by a per-attempt timeout sized to cover a cold TLS
handshake to the resolver over the tunnel (default 8 seconds), and the resolver SHALL
keep a primary DoH endpoint and one or more backup endpoints. On a `429` or a server
error the resolver SHALL rotate to the next endpoint immediately, without waiting out the
timeout; the timeout SHALL apply only to an endpoint that does not respond at all. Every
endpoint SHALL be an IP-literal HTTPS URL so the resolver needs no prior name resolution,
and the endpoint list SHALL be overridable through the tunnel options.

#### Scenario: Timeout falls through to the backup quickly

- **WHEN** the primary DoH endpoint does not answer within the per-attempt timeout
- **THEN** the resolver retries the backup endpoint after at most that short timeout,
  not after a 30-second stall

#### Scenario: Rate limit falls through to the backup

- **WHEN** the primary DoH endpoint answers `429`
- **THEN** the resolver retries the backup endpoint

#### Scenario: Endpoints need no bootstrap resolution

- **WHEN** the resolver connects to a DoH endpoint
- **THEN** it dials an IP-literal HTTPS URL and performs no prior DNS lookup to reach
  the resolver

#### Scenario: A hostname endpoint is rejected, not hung

- **WHEN** a configured DoH endpoint uses a hostname rather than an IP literal
- **THEN** the resolver returns a clear error for that endpoint (resolving it would
  require recursive DNS) rather than stalling until the timeout expires

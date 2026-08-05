# smol-core-stack Specification (delta)

## ADDED Requirements

### Requirement: Truncated DNS responses are rejected

The DNS resolver SHALL reject a response whose truncation (`TC`) bit is set rather
than returning the partial set of answers it happens to carry. A truncated response
is incomplete by definition (RFC 1035 requires retrying over TCP), so it MUST surface
as an error instead of being treated as an authoritative answer.

#### Scenario: Truncated response does not yield partial addresses
- **WHEN** the upstream returns a response with the `TC` bit set
- **THEN** the resolver returns an error rather than the partial answers in that
  response

## ADDED Requirements

### Requirement: Attested retrieval through the anchor's sources

The crate SHALL provide a verified read of the whole geolocation record set at a height `H` through an `AttestedTrustAnchor` whose sources serve geolocation records, with no chain RPC connection. It SHALL obtain the verified records and node identities through the anchor's shared read from its sources, and SHALL return the same verified, grouped shape as the RPC-backed read, with each self-declared entry's attestation checked against the attested identity map.

#### Scenario: Geolocation verified through the anchor's sources
- **WHEN** a quorum agrees on the geolocation snapshot for `H` and a source serves records and node identities for `H` that recompute to it
- **THEN** the read returns the grouped verified set for `H`, with each self-declared entry's attestation checked against the attested identity map

#### Scenario: A source serving bad records is skipped
- **WHEN** the first source serves records that do not recompute to the attested accumulator and the second serves records that do
- **THEN** the read returns the set verified from the second source

#### Scenario: Every source tampered fails closed
- **WHEN** no source serves records and identities that recompute to the attested values
- **THEN** the read returns a verification error and no records

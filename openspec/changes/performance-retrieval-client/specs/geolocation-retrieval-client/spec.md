## ADDED Requirements

### Requirement: Attested retrieval through the anchor's sources

The crate SHALL provide a verified read of the whole geolocation record set at a height `H` through an `AttestedTrustAnchor` whose sources serve geolocation records, with no chain RPC connection: its verified result SHALL be buildable from the record set the anchor's shared read from its sources returns for `H`. The result SHALL be the same verified, grouped shape the RPC-backed and offline reads return, with each self-declared entry's attestation checked against the attested identity map. Which source's data is used, and the failure when none verifies, are the anchor's read's.

#### Scenario: Geolocation verified through the anchor's sources
- **WHEN** a quorum agrees on the geolocation snapshot for `H` and a source serves records and node identities for `H` that recompute to it
- **THEN** the result built from the anchor's read is the one built from the same data through the offline read: the grouped verified set for `H`, with each self-declared entry's attestation checked against the attested identity map

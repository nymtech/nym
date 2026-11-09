## ADDED Requirements

### Requirement: Raw verified records for producers

Beside the grouped read, the client SHALL offer a verified read of the whole record set at a height `H` that returns the records exactly as read, un-grouped, together with the accumulator the verification established and the node-identity map at `H`, so that a producer can re-serve what it verified and sign the accumulator and identity hash it was given. It SHALL perform the same height-pinned read and the same digest verification as the grouped read, and the grouped read SHALL be built from its result, so the two cannot diverge. It SHALL fail closed exactly as the grouped read does, returning an error and no records.

#### Scenario: A producer re-serves what the raw read returned
- **WHEN** a producer takes the raw verified read at `H`
- **THEN** the returned records, the whitelisted-agent records among them, recompute to the returned accumulator, and the grouped read at `H` is the grouping of those same records under the returned identity map

#### Scenario: The raw read fails closed
- **WHEN** the records read at `H` do not recompute to the digest the anchor trusts for `H`
- **THEN** the raw read returns a verification error and no records, as the grouped read does

### Requirement: HTTP attestation source for geolocation

The crate SHALL provide a concrete HTTP `AttestationSource` whose record type is `GeolocationRecord`, fetching from a nym-api producer, so that an `AttestedTrustAnchor` can reach quorum over real producers' geolocation snapshots. It SHALL hold a nym-api client and the signer identity expected from that producer, SHALL fetch the producer's latest signed geolocation snapshot and its signed geolocation snapshot at a height from the `/v1/geolocation` routes, and SHALL assemble the snapshot data at a height from every page of the geolocation records at that height and the node-identity map the producer's directory tree serves at the same height, which is the map the geolocation snapshot hashes. Its nym-api client methods SHALL be provided by the validator client's nym-api client trait, beside the directory's and performance's. Transport or decode failures SHALL surface as a typed transport error, never as an absent or empty answer, and so SHALL records pages that add nothing before the announced total is reached or overrun it, so that no producer can keep the source paging forever.

#### Scenario: Anchor reaches quorum over geolocation producers
- **WHEN** an `AttestedTrustAnchor` is configured with HTTP geolocation sources pointing at K trusted producers that agree on a snapshot for `H`
- **THEN** the anchor reaches quorum over their signed snapshots and its shared read yields the same verified result as the RPC path at `H`

#### Scenario: Snapshot data joins the records with the directory's identities
- **WHEN** a producer's geolocation records at `H` span several pages
- **THEN** the source returns every record of every page together with the node-identity map from `/v1/directory/entries/H/node-identities`

#### Scenario: Transport failure is typed
- **WHEN** a producer is unreachable, returns undecodable data, or has no identity map for `H`
- **THEN** the source returns a typed transport error, and the anchor treats it as a non-answer

#### Scenario: Pages that never complete fail the read
- **WHEN** a producer's records pages at `H` announce more records than they ever serve
- **THEN** the source returns a typed transport error rather than paging forever

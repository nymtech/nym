## ADDED Requirements

### Requirement: HTTP attestation source for performance

The crate SHALL provide a concrete HTTP `AttestationSource` whose record type is `LatestPerformance`, fetching from a nym-api producer, so that an `AttestedTrustAnchor` can reach quorum over real producers' performance snapshots. It SHALL hold a nym-api client and the signer identity expected from that producer, SHALL fetch the producer's latest signed performance snapshot and its signed performance snapshot at a height from the `/v1/performance` routes, and SHALL assemble the snapshot data at a height from every page of the performance records at that height and the node-identity map the producer's directory tree serves at the same height, which is the map the performance snapshot hashes. Its nym-api client methods SHALL be provided by the validator client's nym-api client trait, beside the directory's. Transport or decode failures SHALL surface as a typed transport error, never as an absent or empty answer.

#### Scenario: Anchor reaches quorum over performance producers
- **WHEN** an `AttestedTrustAnchor` is configured with HTTP performance sources pointing at K trusted producers that agree on a snapshot for H
- **THEN** the anchor reaches quorum over their signed snapshots and its shared read yields the same verified result as the RPC path at H

#### Scenario: Snapshot data joins the records with the directory's identities
- **WHEN** a producer's performance records at H span several pages
- **THEN** the source returns every record of every page together with the node-identity map from `/v1/directory/entries/H/node-identities`

#### Scenario: Transport failure is typed
- **WHEN** a producer is unreachable, returns undecodable data, or has no identity map for H
- **THEN** the source returns a typed transport error, and the anchor treats it as a non-answer

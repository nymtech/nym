## MODIFIED Requirements

### Requirement: Produce only what was verified

Before signing a snapshot for height `H`, a producer SHALL fetch and verify the whole directory at `H` through a `DirectoryTrustAnchor`-backed retrieval, and SHALL sign only the `app_hash`, digest `accumulator`, and node-identity hash that verification established. The source anchor SHALL be configurable (defaulting to a proven-RPC anchor against the producer's own chain connection, and permitting a light-client anchor), and SHALL NOT be an attested anchor (which would make the producer trust another attestation to make its own). When the light-client source anchor is selected, the producer SHALL obtain its seed checkpoint from the checkpoint-bootstrap layer rather than from an unimplemented path, and SHALL persist the anchor's light-client-verified head so a restart within the trusting period reseeds from that head without needing a fresh checkpoint.

#### Scenario: A directory that fails verification is not attested

- **WHEN** the producer's source anchor fails to verify the directory at `H` (for example the locally recomputed accumulator does not match)
- **THEN** the producer does not sign or publish a snapshot for `H`

#### Scenario: Operator can choose the source anchor

- **WHEN** an operator configures the producer with a light-client source anchor instead of the default proven-RPC anchor
- **THEN** the producer verifies the directory via that anchor before signing, and the published snapshot format is unchanged

#### Scenario: Light-client source anchor is bootstrapped, not stubbed

- **WHEN** an operator selects the light-client source anchor
- **THEN** the producer loads and verifies a root-signed checkpoint via the checkpoint-bootstrap layer and constructs the anchor from it, instead of failing on an unimplemented checkpoint-retrieval path

#### Scenario: Producer recovers across restart without a fresh checkpoint

- **WHEN** a producer using a light-client source anchor restarts after being down for less than the trusting period
- **THEN** it reseeds the anchor from its persisted verified head and resumes producing without requiring a newly minted checkpoint

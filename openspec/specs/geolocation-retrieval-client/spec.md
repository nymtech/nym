# geolocation-retrieval-client Specification

## Purpose
TBD - created by archiving change geolocation-retrieval-client. Update Purpose after archive.
## Requirements

### Requirement: Verified whole-set geolocation retrieval

The client SHALL retrieve every geolocation record at a single block height and, before returning anything, recompute the `LtHash16` accumulator over each record's `GeolocationRecord::digest_leaf()` and compare it to a trusted digest obtained from a `TrustAnchor`. On any mismatch it MUST return an error and MUST NOT return partial or unverified records as if verified.

#### Scenario: Successful verified retrieval

- **WHEN** the recomputed accumulator over all records read at height `H` equals the anchor's trusted accumulator for `H`
- **THEN** the client returns the grouped record set together with `H` and the verified accumulator

#### Scenario: Tampered record set is rejected

- **WHEN** any record differs from what the accumulator at `H` commits
- **THEN** the client returns a digest-mismatch error and no records

### Requirement: Single-block-height read consistency

The digest read and every paginated record page MUST be executed at one fixed block height `H`. `get_all_geolocation_records` currently pages without pinning a height, so the client SHALL use a height-pinned pagination path instead.

#### Scenario: Pagination is pinned to one height

- **WHEN** the record set spans multiple pages
- **THEN** every page request is issued at `height = H`, the same height the trusted digest was established at

#### Scenario: A concurrent submission does not corrupt the result

- **WHEN** an agent submits a measurement at a height later than `H` while pagination is in progress
- **THEN** the returned set still matches the accumulator committed at `H`, rather than failing verification because pages straddled the write

### Requirement: The verified whitelist authorises measured entries

The agent whitelist is its own entry class inside the same accumulator, so a successful recompute authenticates it alongside the records. The client SHALL therefore determine which agents were authorised at `H` from the verified whitelist, and SHALL NOT accept a whitelist from any other source.

#### Scenario: Measured entry from an authorised agent

- **WHEN** a measured entry's writing agent appears in the verified whitelist with `can_measure`
- **THEN** the entry is reported as written by an authorised agent

#### Scenario: Fabricated whitelist cannot be substituted

- **WHEN** a source supplies a whitelist that differs from the one the accumulator commits
- **THEN** the recompute fails and the client returns an error, so a forged authorisation set cannot launder measured entries

### Requirement: Per-class verification semantics

Each entry class carries a different authority and the client SHALL apply the matching check. A measured entry carries no signature and is authorised by whitelist membership. A self-declared entry SHALL have its node ed25519 attestation verified over the declaration's signing payload against the subject node's identity key. An admin override carries no further check, since the accumulator is the whole of its authority.

#### Scenario: Self-declared attestation verifies

- **WHEN** a self-declared entry's signature verifies against the subject node's identity key
- **THEN** the entry is reported as node-attested

#### Scenario: Self-declared attestation fails

- **WHEN** a self-declared entry's signature does not verify
- **THEN** the entry is reported as unauthenticated rather than presented as node-attested

#### Scenario: Override needs no signature

- **WHEN** an entry's source is an admin override
- **THEN** no per-entry signature is required or checked

### Requirement: An entry from a de-authorised agent is reported, not dropped

The contract enforces the whitelist at write time, so a stored measured entry whose agent is absent from the whitelist at `H` can only have arisen from later de-authorisation. The client SHALL return such an entry marked as written by an agent no longer authorised at `H`, rather than silently discarding it.

#### Scenario: Agent removed after writing

- **WHEN** a measured entry exists at `H` whose writing agent is not in the verified whitelist at `H`
- **THEN** the entry is returned with its authority recorded as de-authorised, so a consumer can decide for itself whether to use it

### Requirement: An undecodable payload verifies and is returned

Verification operates on raw bytes via `digest_leaf`, so it is independent of payload decoding. A payload of a version this client cannot decode SHALL still verify, and SHALL be returned with its raw bytes and no decoded location, never dropped.

#### Scenario: Future payload version survives

- **WHEN** an entry carries a payload version this client does not understand
- **THEN** the record set still verifies against the accumulator, and that entry is returned with its raw payload and no decoded location

#### Scenario: A payload bump does not silently empty the set

- **WHEN** every entry for a node uses a payload version this client cannot decode
- **THEN** the node is still present in the returned set with its undecodable entries, rather than appearing to have no geolocation data at all

### Requirement: Retrieval returns every entry, grouped per subject

The verified read SHALL return all entries grouped by subject, with each node carrying its measured entries, its self-declared entry, and its override separately, and each entry carrying its method, agent where applicable, `checked_at`, decoded location where decodable, and authority status. The client SHALL NOT collapse them into a single answer.

#### Scenario: Disagreeing agents are both visible

- **WHEN** two authorised agents have written different locations for the same node
- **THEN** both entries are returned, each with its own agent, `checked_at` and authority, and neither is discarded

### Requirement: Resolution policy is pluggable with a documented default

The crate SHALL expose a policy seam that selects one of a subject's already-verified entries, and SHALL ship one default implementation. The default SHALL prefer an admin override, then a measured entry, then a self-declared entry, on the grounds that a self-declaration is a node asserting about itself and is unverifiable by a third party.

Among measured entries the default SHALL select the two-letter country code that the greatest number of them agree on, and then the freshest of those by `checked_at`. Agreement SHALL be judged on the country code alone, since coordinates, city and organisation differ between providers for a node that has not moved. The default SHALL apply no maximum age: an entry the contract holds is something to fall back on, and ageing one out would drop a subject to a weaker source, or to none, because no agent has swept it recently - a fact about the sweep rather than about the subject.

Selection requires a location, so the default SHALL only select an entry whose payload it can decode. An undecodable entry SHALL cause precedence to fall through to the next class, except for an override, which SHALL resolve to nothing rather than fall through to the value it was set to suppress. A self-declared entry SHALL additionally be selected only if its attestation verified.

A caller SHALL be able to supply its own policy without reimplementing verification, including one that applies a freshness bound of its own.

#### Scenario: Default precedence applied

- **WHEN** a node has both a measured entry and a self-declared entry, both selectable
- **THEN** the default policy selects the measured entry

#### Scenario: Self-declaration as fallback

- **WHEN** a node has only a self-declared entry and its attestation verified
- **THEN** the default policy selects it

#### Scenario: Agreement outranks freshness

- **WHEN** a node has one recent measurement naming one country and several older measurements agreeing on a different one
- **THEN** the default policy selects from the country the greater number agree on, not the most recent entry

#### Scenario: Freshest of the agreeing measurements is chosen

- **WHEN** several measurements agree on a country but carry different `checked_at` values
- **THEN** the default policy selects the freshest among them

#### Scenario: Nothing expires

- **WHEN** a node's only measured entry is arbitrarily old
- **THEN** the default policy still selects it, because stale data is more useful than none

#### Scenario: An undecodable entry cannot win its slot

- **WHEN** a node's only measured entry carries a payload version this build cannot decode, and the node also has a verified self-declaration this build can decode
- **THEN** the default policy falls through and selects the self-declaration, and the undecodable measurement remains present in the returned set

#### Scenario: An undecodable override suppresses rather than falls through

- **WHEN** a node has an override this build cannot decode and a decodable measurement
- **THEN** the default policy selects nothing, rather than serving the value the override was set to suppress

#### Scenario: Caller substitutes its own policy

- **WHEN** a consumer supplies its own policy implementation
- **THEN** it receives each subject's verified entries including authority status, and its selection is used in place of the default

### Requirement: Selection does not synthesize

A policy SHALL select among entries that exist rather than construct a new location. A consumer wanting to derive a value across entries has the whole verified set and does not need the policy seam.

#### Scenario: Returned value is traceable to one entry

- **WHEN** a policy resolves a subject
- **THEN** the result identifies the specific entry chosen, carrying its provenance, rather than a value that corresponds to no stored entry

### Requirement: Attested retrieval without a chain RPC connection

The crate SHALL support verifying a whole-set fetch using only a quorum-attested snapshot and local recompute, with no chain RPC connection. Node identities needed for self-declared attestations SHALL come from the snapshot data and be checked against the attested `node_identities_hash`, rather than from a chain query.

#### Scenario: No-RPC client verifies geolocation from a producer

- **WHEN** a client fetches the geolocation record set and node identities for a retained height `H` from a producer, and holds a quorum-attested snapshot for `H`
- **THEN** it verifies the records against the accumulator and the identities against the `node_identities_hash` by local recompute alone

#### Scenario: Mismatch fails closed

- **WHEN** the fetched data does not recompute to the attested accumulator
- **THEN** the client returns a verification error and no records

### Requirement: Height alignment with the directory

A consumer joining directory and geolocation data SHALL be able to pin both to one height. The geolocation attestation source and its route shape SHALL therefore address the same cadence-boundary heights the directory producer retains.

#### Scenario: One height serves both trees

- **WHEN** a consumer holds a directory snapshot at retained height `H`
- **THEN** it can request a geolocation snapshot and record set at the same `H` and verify both, rather than having to join views taken at different heights

### Requirement: Proven single-entry reads are out of scope

The entries key is `(subject_class, subject_id, source)`, so retrieving a subject's entries without knowing the source is a prefix scan, and ICS23 proves membership and non-membership of a key rather than completeness over a range. The crate SHALL NOT offer a proven read that appears to answer "all entries for this subject", and completeness for a subject SHALL be established only by the whole-set recompute.

#### Scenario: Completeness comes from the whole-set path

- **WHEN** a caller needs every entry for one subject with an assurance that none was withheld
- **THEN** the whole-set verified read is the only path that provides it

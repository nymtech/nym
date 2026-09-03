# tendermint-light-client-anchor Specification

## Purpose

Defines the requirements for `LightClientAnchor`, a `DirectoryTrustAnchor` implementation that verifies block headers via the Tendermint light-client protocol (validator-set consensus) before trusting the `app_hash` they carry. It replaces the honest-RPC assumption of `ProvenTrustAnchor` with a cryptographic guarantee: the returned `app_hash` is accepted only if >2/3 of the known validator set signed the block that contains it.

## Requirements

### Requirement: Trusted checkpoint at construction
`LightClientAnchor` SHALL be constructed from a caller-supplied trusted checkpoint (`Checkpoint { height, signed_header, validators, next_validators }`) that represents a block the caller has verified through an out-of-band channel (e.g., a genesis-pinned block or an operator-attested recent block). The anchor SHALL NOT fetch or self-bootstrap the checkpoint from the RPC it is given at construction.

#### Scenario: Valid checkpoint initialises anchor
- **WHEN** a caller constructs `LightClientAnchor::new(client, directory_contract, checkpoint, options)`
- **THEN** the anchor stores the checkpoint's validator set as the initial trusted state and begins sequential verification from `checkpoint.height`

#### Scenario: No RPC call is made during construction
- **WHEN** `LightClientAnchor::new` is called
- **THEN** it does not make any network call; all RPC interaction is deferred to the first `trusted_app_hash` or `trusted_digest` call

### Requirement: Header verification via validator-set consensus
`LightClientAnchor::trusted_app_hash(H)` SHALL verify the signed header at `H+1` against the most recent trusted validator set using the Tendermint light-client verification rule (>2/3 of the trusted next-validators must have signed the block). The `app_hash` from `header[H+1]` is returned ONLY after this verification passes.

#### Scenario: Valid header passes verification
- **WHEN** the signed header at `H+1` is signed by more than 2/3 of the trusted validator set
- **THEN** `trusted_app_hash(H)` returns `header[H+1].app_hash` and updates the trusted state to `H+1`

#### Scenario: Invalid or insufficiently signed header is rejected
- **WHEN** the signed header at `H+1` is signed by ≤2/3 of the trusted validator set, or the signature set is malformed
- **THEN** `trusted_app_hash(H)` returns an error and does NOT return an `app_hash` or update trusted state

#### Scenario: Forged header from adversarial RPC is rejected
- **WHEN** the RPC returns a header at `H+1` whose signatures do not match the trusted validator set
- **THEN** verification fails and no `app_hash` is returned, preventing a forged proof from passing downstream ICS23 checks

### Requirement: Skip verification with bisection fallback
When the trusted state is at height `T` and `trusted_app_hash(H)` is called with `H+1 > T+1`, the anchor SHALL first attempt to verify `H+1` directly from `T` (skip verification). If the voting power that the trusted validator set contributes to the commit at `H+1` meets the trust threshold (≥1/3), the block is accepted in one shot. If the overlap is insufficient, the anchor SHALL bisect: verify the midpoint `M = (T + H+1) / 2` from `T`, advance the trusted state to `M`, then retry `H+1` from `M`, recursing until the target is reached. The anchor MUST NOT require the caller to supply a checkpoint close to the target height.

#### Scenario: Direct skip succeeds (stable validator set)
- **WHEN** the trusted state is at `T`, `trusted_app_hash(H)` is called with `H >> T`, and ≥1/3 of the trusted validator voting power signed the block at `H+1`
- **THEN** the anchor verifies `H+1` in a single direct check, updates trusted state, and returns `app_hash` without fetching any intermediate block

#### Scenario: Bisection triggers on insufficient overlap
- **WHEN** the direct verification of `H+1` from `T` fails due to <1/3 trusted voting power overlap
- **THEN** the anchor verifies the midpoint `M` first, then retries `H+1` from `M`, requiring at most O(log delta) verification steps total

#### Scenario: Target height already verified
- **WHEN** `trusted_app_hash(H)` is called for an `H` whose `H+1` app hash is already in the cache
- **THEN** the cached `app_hash` is returned immediately without any RPC call

### Requirement: In-memory cache of verified app hashes
Verified `(Height, AppHash)` pairs SHALL be cached in memory within the anchor instance. Repeated calls to `trusted_app_hash(H)` for the same `H` within one process lifetime SHALL return the cached value without re-fetching or re-verifying.

#### Scenario: Repeated query uses cache
- **WHEN** `trusted_app_hash(H)` is called twice for the same `H` in the same session
- **THEN** only one round of header fetching occurs; the second call returns from cache

### Requirement: Trusted digest via anchor
`LightClientAnchor::trusted_digest(H)` SHALL establish the trusted digest at `H` by calling `trusted_app_hash(H)` and then proving the on-chain `digest_state` item via an ICS23 membership proof verified against that `app_hash`. This is identical in structure to `ProvenTrustAnchor::trusted_digest` except the `app_hash` is header-verified.

#### Scenario: trusted_digest succeeds when header verifies
- **WHEN** `trusted_app_hash(H)` succeeds and the ICS23 proof of the digest item verifies against it
- **THEN** `trusted_digest(H)` returns the proven `TrustedDigest` value

#### Scenario: trusted_digest fails when header does not verify
- **WHEN** `trusted_app_hash(H)` fails (header not adequately signed)
- **THEN** `trusted_digest(H)` propagates the error

### Requirement: Configurable verification options
The caller SHALL supply `tendermint_light_client_verifier::Options` at construction, including `trusting_period` and `clock_drift`. The anchor SHALL apply these options to every `verify_update_header` call and SHALL fail if the trusted block is outside the trusting period relative to the current wall-clock time.

#### Scenario: Checkpoint within trusting period passes
- **WHEN** the trusted block's timestamp is within `now - trusting_period` and the header being verified is valid
- **THEN** verification succeeds

#### Scenario: Stale checkpoint beyond trusting period is rejected
- **WHEN** the trusted block's timestamp is older than `trusting_period` relative to the current time
- **THEN** verification returns an error before any `app_hash` is returned

### Requirement: Feature-gated compilation
`LightClientAnchor` SHALL be compiled only when the `light-client` feature is enabled on `nym-directory-client`. The `ProvenTrustAnchor` and all other crate functionality SHALL remain available without the feature.

#### Scenario: Crate builds without the feature
- **WHEN** `nym-directory-client` is compiled without `features = ["light-client"]`
- **THEN** the crate compiles and `ProvenTrustAnchor` is available; `LightClientAnchor` is not

#### Scenario: Crate builds with the feature
- **WHEN** `nym-directory-client` is compiled with `features = ["light-client"]`
- **THEN** `LightClientAnchor` is available alongside `ProvenTrustAnchor`

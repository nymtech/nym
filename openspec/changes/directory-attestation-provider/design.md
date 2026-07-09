## Context

`directory-attested-anchor` shipped `AttestedTrustAnchor<S>`, a `DirectoryTrustAnchor` that trusts a K-of-N quorum of nym-api identity keys signing a `DigestSnapshot { chain_id, directory_contract, height, app_hash, accumulator, node_identities_hash }`. It defined the canonical signing encoder (`digest_snapshot_signing_payload`) and the node-identity hash (`node_identities_hash`), but kept them `pub(crate)` inside `nym-directory-client` and tested the anchor only against a mock `AttestationSource`. Its Non-Goals explicitly deferred three things to a follow-up (this change): the nym-api producer that signs snapshots, a concrete HTTP `AttestationSource`, and generalized canonical-subset attestation. It also left, as open questions, the snapshot cadence and the retained-window size (leaning "contract-side for consistency").

Two facts from the current codebase shape this change. First, nym-api already has everything a producer needs: a stable ed25519 identity keypair in `AppState.identity_keypair`, an in-memory `DescribedNodes` cache of node keys/addresses, a chain client (`nym-api/src/support/nyxd`) that reads block headers/`app_hash` and does contract queries, and validator-client plumbing that already knows the directory and mixnet contract addresses (`DirectoryQueryClient`, `directory_contract_address()`). Second, there is a close precedent for the wrapper shape: `nym-node-requests` already has `SignedData<T> { data, signature }` (a nym-node signs its own `HostInformation` - keys + addresses - as `SignedHostInformation`), which nym-api polls and re-serves. The generic subset envelope here is the directory-scoped, quorum-capable generalization of that pattern.

The core problem the cadence solves: a K-of-N quorum can only agree on a snapshot if independent nym-apis produce snapshots at the *same* heights. Left to each api's own "latest block", they would almost never share an exact height and the anchor's confirm step (`snapshot_at(H)` on peers) would starve. A shared, contract-dictated cadence makes the produced heights deterministic and identical across all apis.

## Goals / Non-Goals

**Goals:**

- Extract the attestation protocol into a signer-agnostic library (`nym-directory-attestation`) that a producer (nym-api now, nym-node later) and the verifying client both depend on, without either owning the other.
- Make nym-api a producer: read the contract cadence, produce and sign `DigestSnapshot`s at cadence heights, retain a small window, and serve them over HTTP - so `AttestedTrustAnchor` reaches quorum against real nym-apis.
- Add a generic, quorum-verifiable mechanism for signed "subsets" of directory/node data, as reusable scaffolding, without committing to any concrete production subset yet.
- Add a contract-dictated snapshot cadence so independent producers converge on identical heights.
- Let a nym-api serve the whole verified directory at a retained height, so the prior change's `verify_directory_offline` has a real no-RPC server to pull from.
- Provide the concrete HTTP `AttestationSource` (`#4`) and the client-side subset quorum/verify path.
- Leave `DigestSnapshot`, the `DirectoryTrustAnchor` trait, `ProvenTrustAnchor`, and `LightClientAnchor` behaviorally unchanged; the type move is source-compatible via re-exports.

**Non-Goals:**

- A concrete production subset. The mechanism is generic and exercised only against a dummy test subset; the real projection is decided and wired in a follow-up.
- nym-node as a producer (the library is signer-agnostic and ready; wiring the lower tier is a follow-up).
- Putting retention count or settle-lag on-chain; only the interval is on-chain.
- Folding `node_identities_hash` into the generic subset mechanism (it stays grandfattered inside the anchor snapshot).
- Persisting produced snapshots across nym-api restarts.
- The checkpoint / root-key bootstrap (steps 1b/1c), still postponed.

## Decisions

### D1: A dedicated `nym-directory-attestation` crate is the shared protocol home

`common/directory-attestation/` (package `nym-directory-attestation`, following the `nym-lthash`/`nym-topology`/`nym-types` convention). `DigestSnapshot`, `SignedDigestSnapshot`, `digest_snapshot_signing_payload`, and `node_identities_hash` move here from `nym-directory-client` (they were `pub(crate)`, placement punted in the prior change's D3, which said "the producer, when it lands, can decide then whether to depend on this crate or extract a shared piece"). This is that decision. `nym-directory-client` depends on the new crate and re-exports the moved types from `src/anchor/mod.rs` so no downstream `use` path changes.

Dependencies stay deliberately light so both a producer and the verifier can pull it in: `nym-crypto`, `serde`, `cosmrs`/`tendermint` (`AppHash`, `Height`), `nym-lthash` (with its `serde` + `Hash` features, `LtHash16`), `blake3`, `nym-mixnet-contract-common` (`NodeId`), `async-trait`. Notably **no `nym-validator-client`** (no RPC stack) - that stays a `nym-directory-client` (consumer) concern. The `AttestationSource` trait moves here too (it is the producer/consumer transport contract), while its concrete HTTP impl stays client-side (D9).

Rejected: folding this into `nym-api-requests`. It would make `nym-directory-client` depend on nym-api's request crate, and make a future nym-node producer import nym-api's request crate to *produce* - awkward, since nym-node has its own `-requests` crate. A neutral crate keeps all three consumers depending on something none owns.

### D2: `DigestSnapshot` unchanged; a parallel generic subset mechanism alongside it

The snapshot and a subset do different jobs: the snapshot is a tiny, hash-only trust-anchor bootstrap (quorum-signed, small, so bulk data can be fetched untrusted and re-checked); a subset *is* bulk data the client wants to use. Conflating them into one manifest was considered and rejected (it churns the landed anchor and loses the clean "small commitment vs bulk content" split). So `DigestSnapshot` stays exactly as shipped, and new data rides a separate mechanism:

```
trait DirectorySubset: Sized {                              // a symmetric canonical codec
    const SUBSET_ID: &'static str;
    fn to_canonical_bytes(&self) -> Vec<u8>;
    fn from_canonical_bytes(&[u8]) -> Result<Self, SubsetDecodeError>;
}
struct SubsetDigest       { chain_id, height, subset_id, hash: [u8; 32] }   // hash = subset_hash(id, height, canonical_bytes)
struct SignedSubsetDigest { digest: SubsetDigest, signer: ed25519::PublicKey, signature: ed25519::Signature }
struct AttestedSubset     { signed_digest: SignedSubsetDigest, canonical_data: Vec<u8> }   // non-generic: carries the exact hashed bytes
```

A subset's **canonical bytes are its single wire form** - the same bytes are transported and hashed, so a verifier checks the commitment over exactly what it received rather than a re-encoding of a serde round-trip (this is why `DirectorySubset` is a to/from codec, not just an encoder - see D3a). `node_identities_hash` stays grandfathered inside `DigestSnapshot` (it is load-bearing for the anchor's existing behavior and the full-directory path, D8); only *new* data uses the subset path. Down the road node-identities could be re-expressed as a subset, but not here.

Trust flow (the generalization of the `node_identities_hash` pattern into a first-class mechanism):

```
1. quorum:  ask K-of-N sources for SignedSubsetDigest(H, subset_id) -> reach_quorum on identical `hash`
2. content: fetch ONE AttestedSubset from any source (even untrusted)
3. check:   subset_hash(subset_id, H, &canonical_data) == digest.hash == quorum-agreed hash
4. decode:  T::from_canonical_bytes(&canonical_data)   // only after (3) passes
```

Invariant (spec-enforced): the recompute in step 3 is load-bearing (tampered data fails closed) and is computed over the exact received bytes; a single `SignedSubsetDigest` never confers trust on its own - K distinct trusted signers on the same hash are always required.

### D3: `AttestedSubset` carries the `SignedSubsetDigest`, not a bare `SubsetDigest`

For ~96 bytes over the bulk data, the single response is self-verifying (verify signature over the digest, recompute hash over the carried bytes), and its embedded `signed_digest` **reuses as exactly one quorum candidate** - so a client can fetch data + one vote from one source and ask only the *others* for the remaining votes. This mirrors the anchor's `refresh()` seed-reuse (fetch from one source, count it as a candidate, ask peers for the rest). The embedded digest gets no special trust for being the data-server: it is counted identically to a separately-fetched `SignedSubsetDigest`, and K distinct trusted signers are still required.

### D3a: `DirectorySubset` is a symmetric codec; canonical bytes are the single wire form

`AttestedSubset` carries `canonical_data: Vec<u8>` (not a typed `data: T` transported via serde). A subset's canonical encoding is therefore the *only* encoding - both what crosses the wire and what is hashed - so a verifier recomputes `subset_hash` over exactly the bytes it received (step 3) and then decodes the typed value via `DirectorySubset::from_canonical_bytes` (step 4, `SubsetDecodeError` on malformed input). The rejected alternative (`data: T` over serde JSON, hash recomputed by re-encoding `serde -> T -> canonical_bytes`) leaves two parallel encodings that a subset author must keep in lockstep and verifies the commitment against a re-encoding rather than the received bytes - the wrong trade for a verifiable-retrieval system. The only cost is subset payloads are opaque bytes in JSON rather than typed JSON; fine, since nym-api still serves the same data typed via its normal endpoints and the attested-subset path exists for verification, not introspection.

### D4: Subsets are scaffolding only in this change

No concrete production subset. The keys+addresses example raised during design is a poor fit as-is: `SkimmedNodeV1` carries nym-api-computed `performance` and `role`, which differ across independent apis and over time and would break byte-identical quorum in step 1. A real subset needs a purpose-built, stable projection (just the durable fields), which is a product decision deferred to a follow-up. This change delivers the generic mechanism (types, trait, producer sign-core, client quorum/verify-core) exercised against a **dummy test subset**, ready to instantiate later. Consequently no concrete subset HTTP routes are added to nym-api here.

### D5: Contract-dictated snapshot cadence, as a plain on-chain `Item`

`nym-directory-contract` gains `snapshot_interval` (blocks between snapshots): a plain storage `Item`, set at instantiate (validated positive) and mutable only by the admin (`UpdateSnapshotInterval`, gated exactly like the existing admin ops), read back via a `SnapshotInterval` query. Producers read it to compute cadence heights. Breaking change, acceptable since the contract is not deployed.

It is deliberately **not** committed into the LtHash digest and **not** added to the signed `DigestSnapshot`:

- The digest commits the *entry set*; the interval is config, not an entry. Folding it in would churn the digest on every interval change and muddy what a digest match means, for no benefit.
- Consistency is already chain-enforced: every api reading the interval from on-chain state at the same height sees the same value. An api whose RPC lies about the interval simply produces at the wrong heights and misses quorum with honest apis - self-defeating, not exploitable. The client never needs the interval (it discovers valid heights from `latest`/the quorum).
- Because it is a first-class `Item` at a known raw key, a paranoid producer running against an untrusted RPC (or a client) can still ICS23-prove it against `app_hash` on demand - verifiability without it being in the digest.

The one residual: right at an interval *change*, apis may briefly disagree on cadence heights (transient quorum starvation). This is a read-timing issue that committing the interval would not fix; mitigated by it being admin-gated + rare, clients retrying, and (if ever needed) requiring changes to land on a boundary.

### D6: Producer height model - deterministic cadence, small retained window, settle-lagged `latest`

- Produce at cadence heights `H` where `H % snapshot_interval == 0`. `app_hash` for `H` comes from `header[H+1]` (matching the proven-anchor rule), so a snapshot for `H` is produced once block `H+1` is seen.
- Retain the last `N` cadence snapshots (nym-api config, default ~3). `snapshot_at(H)` answers any retained cadence height immediately.
- `latest` returns the greatest cadence height `H` with `current_tip >= H + settle_lag`, where `settle_lag` is a small number of **blocks** (nym-api config, default ~5). This settling delay means even the fastest api advertises a height every honest peer has already produced and retained, so the anchor's confirm step (`snapshot_at(seed)` on peers) cannot starve on an api that is a few blocks behind. The freshly-produced height is still directly queryable via `snapshot_at` before it settles as `latest`.

Only `snapshot_interval` must be identical across apis (it comes from the contract). `N` and `settle_lag` are local config: mismatched values still converge as long as everyone retains enough and the seeded height is available on peers. `settle_lag` is a *few blocks*, not an interval - `latest` stays fresh.

### D7: The producer verifies before it attests, via a configurable source anchor

To compute a snapshot at `H`, the producer fetches and verifies the whole directory through a `DirectoryClient` bound to a configurable `DirectoryTrustAnchor`, defaulting to `ProvenTrustAnchor` against the api's own RPC (the api trusts the RPC it operates), swappable to `LightClientAnchor` for an operator who does not. It **must never** be `AttestedTrustAnchor` (circular - attesting based on attestation). This makes the producer eat its own dog food: it can only sign an `accumulator`/`app_hash` its own recompute already verified, so a nym-api cannot attest a corrupt directory (its verify would fail first).

Implementation choice (settled at apply time): either `Box<dyn DirectoryTrustAnchor>` with a small `impl DirectoryTrustAnchor for Box<dyn DirectoryTrustAnchor>` (the trait is `async_trait`, so object-safe), or a generic on the producer with a `ProvenTrustAnchor` default. Leaning `dyn` for config ergonomics; either is fine.

### D8: Full-directory serving rides the anchor's existing values, not the subset mechanism

Because the producer already holds the verified whole directory (entries + the `node_identities` it hashed) at each retained `H`, it retains and serves it. A no-RPC client fetches entries + node identities from a nym-api and verifies them with the prior change's `verify_directory_offline` against the quorum-attested `accumulator` + `node_identities_hash` from the `DigestSnapshot`. This is *not* a subset (it needs no `SubsetDigest`): the anchor already commits both hashes. This is what makes the prior change's decoupled path usable against a real server.

### D9: Concrete HTTP `AttestationSource` in `nym-directory-client`, plus the deferred error variant

`HttpAttestationSource` lives in `nym-directory-client` (the consumer side, which may hold an HTTP client dep): `latest_snapshot()` / `snapshot_at(H)` GET the producer's endpoints and deserialize `SignedDigestSnapshot`; `identity()` returns the configured/looked-up signer key for that URL. The prior change deliberately did not add an attestation-transport error variant ("no call site to validate it against"); this change adds `DirectoryClientError::AttestationTransport` (or similar) now that `HttpAttestationSource` is a real call site, wrapping the HTTP client + decode failure shape. The client HTTP stack choice (reuse `nym-http-api-client` / the existing reqwest path) is settled at apply time.

The client-side subset path also lands here: `quorum_subset_digest<T>(...)` (fetch K `SignedSubsetDigest`, `reach_quorum` on the hash - reusing the anchor's distinct-signer counting) and `fetch_and_verify_subset<T>(...)` (fetch one `AttestedSubset<T>`, recompute + check against the quorum hash), plus the whole-directory-from-a-nym-api fetch feeding `verify_directory_offline` (D8).

### D10: nym-api producer wiring

A producer module owns a periodic task (reusing nym-api's existing cache-refresh cadence pattern) that: reads `snapshot_interval` from the contract via the existing chain client; on crossing a cadence boundary, fetches+verifies the directory at `H` via the configurable source anchor (D7), computes the `node_identities_hash` and `accumulator`/`app_hash`, signs the `DigestSnapshot` with `AppState.identity_keypair`, and stores it (plus the full verified directory) in a retained-window store in `AppState`. HTTP routes serve: settle-lagged `latest` snapshot, snapshot at a retained height, and the full directory at a retained height, following nym-api's versioned route + `utoipa` conventions. `SignedDigestSnapshot` gains whatever `ToSchema`/serde support the routes need (in the attestation crate, or a thin wrapper in `nym-api-requests`). nym-api's identity key is already exposed (`GET /v1/api-status/api-information`) and a possession-challenge already exists, so the signer-discovery prerequisite the prior change flagged is already satisfied.

## Risks / Trade-offs

- **Interval-change transient (D5).** A quorum can briefly starve while apis pick up an interval change at slightly different heights. Mitigation: admin-gated + rare; clients retry; optionally require boundary-aligned changes. Committing the interval would not help (it is a read-timing issue).
- **Cross-api canonical reproducibility.** Quorum on any hash (snapshot or subset) requires byte-identical canonical encodings across independently-running apis. Snapshots already have this (fixed-width/length-prefixed encoder). Any future concrete subset must define a stable projection - explicitly called out (D4) as why keys+addresses is deferred.
- **Producer trusts its own RPC by default (D7).** Acceptable: the api operator runs that RPC, and the quorum (K independent apis) is the actual trust boundary for a client. A paranoid operator swaps in `LightClientAnchor`.
- **HTTP transport error shape is new.** Added with its first real call site (D9) rather than guessed earlier; if the HTTP stack choice changes at apply time, the variant adapts.
- **Format churn.** Minimal: the snapshot encoder is reused verbatim from the prior change; the subset encoder is new but defined once in the shared crate for both producer and consumer.

## Open Questions

- HTTP client for `HttpAttestationSource`: reuse `nym-http-api-client`, or a direct reqwest client? (Apply-time; pick the existing convention.)
- Exact route paths / version prefix for the producer endpoints, and whether `SignedDigestSnapshot` needs a `utoipa` wrapper in `nym-api-requests` or can derive `ToSchema` in the attestation crate.
- `dyn` vs generic for the producer's source anchor (D7). Leaning `dyn`.
- Defaults for retained-window `N` and `settle_lag` (design says ~3 and ~5 blocks; confirm against the sphinx-key rotation range width at apply time).
- Whether `settle_lag` should ever move on-chain if convergence needs tightening (deferred; local config for now).

## Migration Plan

1. Add `snapshot_interval` to the directory contract (storage `Item`, instantiate field, `SnapshotInterval` query, admin `UpdateSnapshotInterval`, migration) and wire it through the downstream instantiate paths.
2. Create `nym-directory-attestation`; move the snapshot types + encoders in (with their tests); add the subset types/trait/encoder + producer sign-core (+ dummy-subset tests).
3. Point `nym-directory-client` at the new crate, delete the moved code, re-export to preserve paths; add `HttpAttestationSource` + the transport error variant + the client subset quorum/verify path + the whole-directory-from-nym-api fetch.
4. Add the nym-api producer module + HTTP routes + `AppState` wiring + openapi.
5. Document in the crate README how a producer is wired and how the client consumes snapshots, subsets, and the full directory.

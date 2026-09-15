# Design

## Context

The geolocation contract was built to be verified. It writes its LtHash accumulator with `store.set(DIGEST_STATE.as_bytes(), ..)` at a raw, un-namespaced key, and the doc comment on that constant already records the three invariants a client depends on: the key is used verbatim so the proven key is exactly those bytes appended to the contract's storage prefix, the value is the accumulator rather than its 32-byte collapse, and the key never changes across migrations. What was missing was a client, and task 7.2 of the archived `verifiable-node-geolocation` change says why: `common/nym-directory-client` was not on develop, so the anchors had nowhere to come from.

That crate is now on develop, complete with proven, light-client, checkpoint-bootstrap and attested anchors. Measured in code lines excluding tests, roughly 1230 of it is domain-neutral and roughly 810 is directory-specific, and `proof.rs` contains zero directory references. The expensive parts, a Tendermint light client with bisection and the root-key checkpoint bootstrap, are all in the neutral half.

Two consumers are waiting: nym-api is to serve attested geolocation the way it already serves the directory, and node-status-api is to drop its own metered ipinfo sweep. Neither is in this change.

## Goals / Non-Goals

**Goals:**

- One verifying client for geolocation, whose whole-set read proves completeness against the on-chain accumulator.
- One copy of the anchor and proof machinery in the tree, shared by both contract clients.
- An explicit, replaceable resolution policy, since the contract deliberately stores opinions rather than a verdict.
- A wire and route shape fixed now, so the later nym-api producer implements against a client that already exists.

**Non-Goals:**

- The nym-api geolocation producer. It will consume this crate the way `DirectoryDataProvider` consumes `nym-directory-client`.
- The node-status-api migration, which has its own decisions recorded in `docs/geolocation/node-status-api-migration.md`, notably what a node with no entry means at the dVPN country-code filter.
- Proven single-entry reads. See the decision below.
- Any change to the contract. Nothing here needs one.

## Decisions

### Extract rather than copy or depend

Copy-and-adapt was the right call for the contract's digest machinery, where the duplicate was about a hundred lines inside a wasm binary with its own MSRV and no-std constraints. It is the wrong call here: the shared surface is an order of magnitude larger and includes a light client, and two hand-synchronised light clients is a maintenance liability with no compensating benefit.

Having `nym-geolocation-client` depend on `nym-directory-client` was rejected too. It inverts the dependency for no domain reason and forces `DirectoryClientError` into geolocation's error surface.

So: `common/nym-contract-anchor` holds `proof.rs`, the `TrustAnchor` trait, `TrustedDigest`, the proven, light-client, checkpoint and attested anchors, and the core error. `nym-directory-client` keeps its client, verify core, keys, subsets, fetcher and test support, and re-exports the moved items so its public surface does not move.

### The attestation types need renaming, not generalising

`DigestSnapshot` already carries chain id, contract address, height, `app_hash`, an `LtHash16` accumulator and `node_identities_hash`. Geolocation needs every one of those in exactly that shape: it accumulates `LtHash16` too, and it needs the identity map because self-declared entries carry a node ed25519 attestation. The only domain-specific thing in the type is the field name `directory_contract`.

Cross-contract replay is already prevented, because the contract address is bound into the signing payload. The domain tag separates snapshot signatures from node signing-payload signatures, not directory from geolocation, so no second tag is needed. `DirectorySnapshotData` becomes `SnapshotData<R>`, worth making generic because the fiddly part is the `serde_as` treatment of the identity map rather than the struct.

The crate becomes `nym-contract-attestation`. `nym-attestation` was considered and rejected as vaguer than the content justifies: the only production users are contract attestations. The counter-argument, noted rather than acted on, is that `SubsetDigest` carries no contract address and so is chain-scoped rather than contract-scoped, which would sit oddly if a non-contract subset ever appeared. Every `DirectorySubset` implementor in the tree today is a test dummy, so there is no evidence of that need and speculative reuse is a poor reason to pick a vaguer name.

### One trust root, renamed

`with_default_anchor` reads `DIRECTORY_ATTESTATION_SOURCES` and derives `majority_quorum` from its size, so growing the set moves 2-of-2 to 2-of-3 with no code change. The same nym-apis at the same base URLs sign both contracts' snapshots, so this is one list. It becomes `CONTRACT_ATTESTATION_SOURCES`.

### Verification is whole-set, and that is not a limitation to work around

The entries key is `(subject_class, subject_id, source)`. A directory single-entry read is well-defined because `(node_id, label)` fully determines the key, but a consumer asking where node 42 is does not know the source, so answering is a prefix scan. ICS23 proves membership and non-membership of a key; it does not prove completeness over a range. A proven single-entry read could therefore only answer for a key the caller already knows in full, such as the entry a named agent wrote by a named method.

Offering it anyway would put two different trust arguments behind similarly-shaped calls. Completeness for a subject comes from the whole-set recompute or not at all, and that is what the consumers actually need.

Alternative considered: a prefix read verified by whole-set recompute rather than by proof, presented alongside the proven single-entry read. Rejected for exactly the reason above.

### The whitelist is the point

Measured entries carry no signature, so the natural question is what authorises them. The answer is that the agent whitelist is its own entry class inside the same accumulator, so one successful recompute authenticates the records and the authorisation set together. A client cannot be shown a fabricated whitelist, which is what closes the forgery risk the contract design identified: forgery risk is whitelist addition, omission is only censorship.

A measured entry whose agent is absent from the whitelist at `H` is reported as de-authorised rather than dropped. The contract enforced the whitelist at write time, so that state can only arise from later de-authorisation, and which of those a consumer wants to trust is a policy question, not a verification one.

### Verification is independent of payload decoding

The recompute runs over raw bytes via `digest_leaf`. A version 2 payload will therefore verify perfectly and fail `try_decode_v1`. Such an entry must be returned with its raw bytes and no decoded location.

This is the highest-value test in the change. If it is wrong, a payload bump silently empties the set for every old client, with no error anywhere. It is the same class of silent, delayed failure the workspace `float_roundtrip` pin exists to catch, and it is not hypothetical: the agreed multi-address payload model, `{ v4, v6 }` per family, is a version 2 payload waiting to be written.

### Policy is a seam with a default, not a function

The default precedence is override, then measured, then self-declared. The unobvious half is measured outranking self-declared, and it is the point of the system: a self-declaration is a node asserting about itself and is unverifiable by a third party, so it belongs as a fallback for nodes nothing has measured.

Among measured entries the default takes the country the most of them agree on, and only then the freshest of those. This reverses two earlier decisions in this document, both on implementation review.

Freshest-wins alone was wrong: one recent outlier would overturn ten older measurements that agree with each other, and a lone disagreeing result is far more likely a bad lookup than a relocated node. Plurality was originally rejected on the grounds that it needs three agents to mean anything and there is one - but the tally is over measurements rather than agents, so it already has teeth with a single agent sweeping repeatedly, and it costs nothing while there is only one. Agreement is judged on the two-letter country code alone: coordinates, city and org differ between providers for a node that has not moved, so comparing the whole location would find disagreement everywhere.

Maximum age was dropped rather than parameterised. An entry that is in the contract is something to fall back on, and stale data beats no data: ageing a measurement out drops a subject to a weaker source, or to nothing, because no agent has swept it recently - which is a fact about the sweep, not about the subject. Every entry carries `checked_at`, so a consumer that does want a freshness bound applies one through a replacement policy.

The cost of tallying by country is that selection now depends on payload decoding, which everything else in this change deliberately keeps downstream of verification. Decoding therefore happens once, at retrieval, and is stored on each entry; nothing is filtered on it. An entry this build cannot read cannot answer, so precedence falls through to the next slot that can - except for an override, where falling through would serve exactly the value an admin acted to suppress, so it resolves to nothing instead. The consequence worth stating plainly: a build sees only the votes it can read, so version 2 entries are invisible to an old client's tally even though they remain in the returned set, each carrying its own decode outcome.

The seam selects among entries that exist rather than synthesizing, so its result always traces to one stored entry with its provenance intact. A consumer wanting to derive a value across entries already has the whole verified set and does not need the seam.

### Parallel route trees, aligned heights

nym-api serves the directory under `/snapshot/{latest|height}`, `/subset/{digest|data}/{subset_id}/{height}` and `/{height}/records`. Geolocation gets a parallel tree rather than the live routes being reparameterised into `/attested/{domain}/...`: the directory routes are deployed, the handlers are not shared anyway since the record types differ, and a concrete path keeps the OpenAPI schema specific.

Clients will retrieve both trees, potentially for one logical query, so the producer must use the same cadence and retained-height window for both and one height must serve both. That is a producer requirement, but it is stated here because it is not discoverable from either tree alone. It does not justify a combined endpoint: two trees plus aligned heights lets a consumer join locally, whereas one endpoint returning both record types would undo the reasoning for keeping the trees parallel.

## Risks / Trade-offs

**The extraction silently changes behaviour** → Every pre-existing `nym-directory-client` test must pass unedited except for import paths and renamed identifiers. A test needing a real edit means the move was not mechanical, and that must be resolved before the geolocation crate is built on top.

**Renames ripple further than expected** → Three renames cross crate boundaries: the trait, the error, and the network-defaults constant, plus the crate rename. All consumers are in-tree, so the compiler finds them all; the risk is churn in the diff rather than breakage. Doing the renames as their own commits, before any geolocation code, keeps the mechanical changes separable from the new work.

**Height pinning is easy to get wrong and fails loudly in the wrong place** → An unpinned page read produces a digest mismatch, which looks like tampering rather than like a pagination bug. The fix is a height-pinned pagination path, and the test is a verified read succeeding across a multi-page set while a write commits at a later height.

**Fixtures need a live chain once** → The ICS23 fixtures must be captured from a real chain and then frozen, because unit tests must not depend on the network and blocks get pruned. Sandbox has the geolocation contract deployed with real entries, so both a membership and a non-membership proof can be taken from there.

**The resolution default becomes the de facto public answer** → Whatever node-status-api adopts is what the network sees. Mitigated by making the seam pluggable and by leaving the actual choice to the migration change, where it is visible in that service's own code rather than buried in a library default.

**Two crates could drift on the wire** → `nym-contract-attestation` is shared by both clients and by the future producers, so a change to `DigestSnapshot` affects everything at once. That is the intended trade: one shared type that breaks loudly beats two that diverge quietly.

## Open Questions

None blocking. The producer's route paths are fixed by this change's spec, and the node-status-api decisions are deliberately deferred to that change.

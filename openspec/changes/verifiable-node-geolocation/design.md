## Context

Geolocation currently lives inside `nym-node-status-api` as `monitor/geodata.rs`: an `IpInfoClient`, a `moka` `Cache<NodeId, Location>` with a 24h TTL, and a serial sweep at step 9 of the monitor's strictly-ordered cycle. Results reach consumers by two different routes that can disagree. The dVPN directory reads them out of the persisted `explorer_pretty_bond` JSONB, frozen at gateway-write time; `/explorer/v3/nym-nodes` reads the live in-memory cache, so after a restart it serves `geoip: null` until the sweep refills. Failed lookups are never cached, so a node whose addresses cannot be resolved is retried every cycle against a metered API. An empty country code silently drops a gateway from the dVPN directory (`http/state.rs:431`).

The verifiable-directory stack established the pattern this change follows: a CosmWasm contract maintains an `nym-lthash` accumulator over its own entries and exposes it at a fixed raw storage key, so a client can pull the whole set from an untrusted source, recompute, and prove completeness against the chain's `app_hash` via ICS23 and a light-client anchor. `common/nym-directory-client` implements the anchoring, proving and recompute machinery, in layers of varying generality (see the task 1.2 findings below).

**Merge order matters here.** This change lands on develop *before* `feat/node-directory-publishing`, which is subsequently rebased and remade on top of it. Of the directory stack, only `common/lthash` is on develop today; `common/nym-directory-client`, `common/nym-directory-types` and `contracts/directory/` exist solely on that branch. The contract and the service therefore depend on nothing unmerged, since the accumulator primitive is available and the digest machinery is copy-and-adapted rather than imported. Client-side verification is the one piece that must wait: it has no home until `nym-directory-client` reaches develop. That is a sequencing constraint on the task list, not a gap in the contract, which maintains its digest correctly from the first deployment whether or not tooling exists to check it yet.

Two constraints shape everything below. The key layout and leaf framing are a frozen wire format, committed to by the digest, so changing them is a breaking migration that must re-fold every entry. The payload contents are deliberately kept outside that frozen surface (Decision 10a). And the deferred node status API migration shapes the payload now rather than later, because that API serves two public response shapes derived from the current ipinfo response.

## Goals / Non-Goals

**Goals:**

- Ship independently of `feat/node-directory-publishing`, which merges afterwards and is rebased on top of this work.
- Hold node location in a contract whose full state is provably complete and untampered to an untrusted-source client.
- Make every entry attributable: a reader can tell which agent measured it, or that the node itself signed it, or that the admin set it.
- Support several independent measurement agents concurrently, retaining each one's answer rather than collapsing them.
- Support subjects beyond bonded nym-nodes, so the same contract can hold locations for other NYM infrastructure.
- Design a payload wide enough to serve the node status API's existing public surface, so the follow-up migration needs no contract migration.
- Keep node IP addresses off-chain.

**Non-Goals:**

- **Resolution policy.** The contract stores opinions, not a verdict. With several agents plus a self-declaration plus a possible override, a consumer may see up to five answers for one subject. Choosing between them is the client's job and is deliberately unspecified here.
- **Replacing the node status API's geolocation.** A separate follow-up change, with its own deltas against `node-status-api-monitoring` and `node-status-api-http`.
- **Quorum or slashing over disagreeing agents.** The whitelist is small and NYM-controlled. Retaining per-agent answers makes disagreement detectable, which is enough for now.
- **Extracting shared digest machinery.** See Decision 9.

## Decisions

### 1. A separate contract, not new labels in the directory contract

Structurally the two are near-identical: a composite key to an opaque value under an LtHash digest, with paged enumeration and an unbond callback. The `source` here plays the directory's `label` role, and the subject plays its `EntryKey`. Reusing the directory contract would mean one deployment, one checkpoint and one digest for a client to prove.

Rejected because the auth model is the difference that matters. The directory's entire story is "self-published, self-authenticating from current state alone": every node entry carries the node's own signature over a gap-free sequence, so a client verifies authorisation without trusting anyone. Geolocation is a third party asserting facts about a subject it does not control. Grafting that on would turn a uniform guarantee into "some of this is self-published and some is not, check the label to know which", and would widen the trust surface of a spec that is already archived. Separate contracts keep "who asserted this" answerable from the contract address, and give independent upgrade cadence and blast radius.

### 2. One entries store, keyed by subject class, subject id and a single `Source`

```
  entries   (subject_class, subject_id, source) -> LocationEntry

     subject_class  NymNode                               (closed enum, extensible)
     subject_id     Vec<u8>, encoding fixed per class
     source         Measured { method: Method, agent: Addr }
                  | SelfDeclared
                  | Override

  whitelist (agent_addr) -> AgentPermissions { can_measure, can_relay_self_declared }
```

Three considered alternatives were rejected.

*Three stores split by entry class* (measured / self-declared / override) is honest about their differing key arities, but the cost in a cw-storage-plus contract is per store, not per key component. Each store repeats the whole `StoredNodeEntries` block from `contracts/directory/src/storage.rs:293`: a namespace constant, `storage_key`, prefix constructors, decoders, save/load/range/remove, plus its own leaf arm, paged query and migration surface. Extending a key tuple is a one-line change; adding a store is roughly a hundred lines.

*A flat `(subject, kind, agent_addr)` triple* keeps one store but leaves the agent component vestigial for two of three classes. For a relayed self-declaration the agent is a courier, not a witness, so two agents relaying the same signed artifact would write two byte-identical payloads under different keys, both folded into the digest and neither canonical. For an override the writer is always the admin.

*Separate `kind` and `writer` components* fixes the vestigial-agent problem but leaves the two mutually dependent, so the key space admits combinations that are representable and meaningless: `(subject, SelfDeclared, Agent(a))` and `(subject, Override, Agent(a))` would both need handler-level rejection. Only measured entries have a meaningful writer.

Collapsing them into one `Source` makes invalid states unrepresentable. `Measured` carries both the method and the measuring agent, so each agent keeps its own slot and concurrent agents never overwrite one another. `SelfDeclared` has no writer component at all, so a subject has exactly one self-declared slot no matter which agent relayed it, and conflicting relays are resolved by the `declared_at` monotonicity check in Decision 7. `Override` likewise has one slot, and because it names a role rather than an address, rotating the admin does not orphan existing overrides. Prefix ranges still cover "everything for a subject" and "all measurements for a subject".

The subject splits into a closed-enum class and an opaque id whose encoding is fixed per class, so the contract can hold non-node infrastructure without every id being forced to be a number:

| class | id encoding | rationale |
| --- | --- | --- |
| `NymNode` | `u32` big-endian | preserves numeric ordering, and decodes back to `NodeId` for the unbond callback; a decimal string would sort `"10"` before `"9"` |

`NymNode` is the only class defined. Nothing else is measured yet, so a second class would be speculative; the class component exists so that adding one later is additive rather than a migration. Concretely, adding a nym-api class keyed by its 32-byte ed25519 identity key costs: a new never-reused discriminant, a fixed id width, one arm in the id codec, and whatever authorisation rule that class needs. It costs no leaf-encoding change, no accumulator re-fold and no state migration, because the class lives in the key rather than the value. What it does cost is a redeploy.

`SubjectClass` is a closed enum for the same reason `Method` is (Decision 3). The alternative, an admin-managed string set, trades that redeploy for the risk of a typo silently creating an unreachable subject.

### 3. `Method` is a closed enum

Extensibility is wanted for measurement *sources* (a second vendor, or latency triangulation layered on ipinfo), and that is a single enum variant inside `Source::Measured`. Because it lives in the key rather than the value, a new variant needs no leaf-encoding change at all and therefore no state migration. Subject flexibility, the other axis of extensibility wanted, is handled separately by `SubjectClass`.

An admin-whitelisted string set, as the directory contract uses for labels, would allow new methods without a code change, but buys flexibility on the axis that does not need it while giving up exhaustive matching and typed per-source authorisation rules.

Retiring a variant later is a migration that must subtract every affected leaf from the accumulator before deleting the entries, and the discriminant must be tombstoned rather than reused.

### 4. `checked_at` is committed to the digest leaf

The directory contract deliberately excludes `updated_at_height` from its leaf (`common/cosmwasm-smart-contracts/directory-contract/src/types.rs`, test `node_leaf_excludes_updated_at_height`), so directory write times are not verifiable. This change takes the opposite position, because the requirement that an agent re-submit an unchanged measurement only means something if freshness is provable. Outside the digest, a heartbeat write costs gas, changes nothing a client can check, and an untrusted server serving stale state can still claim it is fresh.

The cost is digest churn on every heartbeat. At current network size and a monthly sweep this is on the order of a hundred entry-writes per day across all agents, which batching reduces to a couple of transactions. The directory contract's `snapshot_interval` concept already establishes that clients verify against snapshot heights rather than every block, so intra-interval churn is not a new problem. If the cadence ever tightens, the escape hatch is committing a coarse bucket (day or epoch index) instead of raw block time, which makes a re-check within the same bucket a genuine no-op the agent can skip submitting.

### 5. Batched submission, amortising the accumulator not the leaf arithmetic

One execute message carries many entries, with a single accumulator load and save for the whole transaction. Each entry keeps its own read-modify-write, because a correct update must subtract the exact bytes of the leaf it is replacing, which requires reading the current value. This also keeps repeated keys within one batch correct.

LtHash is commutative, so batch ordering does not affect the resulting digest. Agents need no canonical ordering and two agents submitting overlapping batches in different orders converge. This property should be stated explicitly so it is not later "fixed" by imposing a sort.

Batches are all-or-nothing, the CosmWasm default. An agent controls what it submits and can pre-validate against the same rules, with one exception: the self-declaration relay path carries data the agent did not produce and whose signature it cannot fully pre-validate against contract state. Self-declaration relays therefore go in their own batches, so one bad signature cannot fail a measurement sweep.

`MAX_BATCH_SIZE` is enforced by the contract and must be chosen by measuring gas against the chain's per-transaction cap, not guessed.

### 6. Authorisation is evaluated at read time, and the whitelist is digest-committed

Measured entries carry no signature from anyone. Their only authorisation is that the sender was whitelisted when the write happened, which a client recomputing the digest cannot see.

*Write-time authorisation* (an entry is valid forever once written by a then-authorised agent) means de-whitelisting a compromised agent leaves its fabricated entries in place, and purging them requires enumerating by agent, which the subject-first key ordering does not support without a full scan or a secondary index.

*Read-time authorisation* (a client intersects entries against the current whitelist) makes compromise recovery instant and free, at the cost that rotating an agent's key silently invalidates its honest history. Compromise recovery is the scenario that should drive this, so read-time wins.

That makes the client's copy of the whitelist load-bearing, so the whitelist is its own digest-committed entry class. The forgery risk is *addition*: an untrusted server supplying a whitelist containing an agent that was never authorised, laundering its fabricated entries. Omission is only censorship, which is less severe. Per-agent ICS23 proofs would defeat forgery without needing set completeness, but cost a round trip per agent and only help the paranoid path; folding the whitelist into the digest is a small amount of code and covers the bulk-pull path where most clients live.

Entries from de-whitelisted agents then linger as digest bloat. A lazy paginated admin purge handles that, as hygiene rather than as a security control.

### 7. Relayed self-declarations are node-signed with monotonic `declared_at`

The node serves a signed `NymNodeLocation` artifact over HTTP and the geolocator relays it verbatim, so the on-chain self-declaration is authenticated by the node rather than by the agent's word.

A bare signature over `(node_id, location)` is replayable forever: anyone who once saw the artifact could resubmit last year's value after the node moved, and the contract could not tell, because the signature is genuinely valid.

```
  payload = domain_tag || node_id || location || declared_at

  accepted iff  declared_at >  stored.declared_at        (strict monotonicity)
          and   declared_at <= block_time + MAX_SKEW     (no far-future pinning)
```

The `location` component of that payload is the stored opaque payload bytes, length-prefixed, never a parsed `Location`. This follows from Decision 10a, since the contract must verify against exactly the bytes it stores, and it is also what lets the contract build the payload without linking the payload types at all (Decision 10). The shared signing-payload helper is therefore byte-level, with the typed `NymNodeLocation` a thin wrapper over it on the producer side.

The directory contract's gap-free sequence approach also works but would require the node to query a contract it otherwise does not touch. A node-supplied timestamp needs no chain read, and doubles as the freshness signal for this kind. `declared_at` is distinct from `checked_at`: the node declared at one time, the agent relayed at another, and both belong in the value.

The far-future guard is load-bearing. Without it, one signature stamped years ahead permanently freezes the slot, because nothing can ever exceed it. `MAX_SKEW` of a few minutes covers worst-case block inclusion plus reasonable clock drift. No lower bound is needed: monotonicity already handles the past, so a node with a slow clock simply advances from wherever it starts. A node whose clock runs ahead by more than the skew is rejected until real time catches up, and that rejection needs a distinguishable error, otherwise it presents as "the geolocator is broken".

### 8. IP addresses are never written on-chain

`NodeInformation` carries both `hostname` and `ip_addresses`, so an operator who wants to stay behind DNS announces a hostname only. The geolocator must resolve that hostname to geolocate it, and writing the resolved address on-chain would deanonymise precisely the operator who opted out. Resolved addresses are transient in the service: never logged durably, never exposed.

Storing a hash of the address set, which would have made change detection stateless, does not work. IPv4 is 2^32, so an unsalted hash is brute-forceable, and a contract-wide salt is public. A hash of an IP is an IP.

The consequence is that change detection is agent-local, best-effort, and lost on restart. A cold-starting agent treats its first sweep as its baseline and only triggers explicit re-tests thereafter; anything that moved during downtime is caught by the next regular sweep. The alternative, re-testing everything on boot, would burn the metered quota on every deployment.

### 9. Copy-and-adapt the digest machinery rather than extracting a shared crate

The verifiable-digest pattern note explicitly blesses either route and observes that distinct `domain_tag`s keep leaves apart, with the ICS23 proof already bound to a specific contract address and storage key. Generic cw-storage-plus wrappers over differing key arities and value codecs cost more than they save at two consumers. Drift between the two implementations is guarded by shared conformance test vectors for the leaf encoding rather than by shared code, since drift between contract and verifier is the failure that actually matters.

### 10. One uniform `Location` payload for every entry, matching the node status API shape

Every entry carries the same location payload regardless of its source. A measurement, a relayed self-declaration and an admin override are all a `Location`, and that `Location` is the shape node status API already serves on its dVPN surface (`http/models/mod.rs:204`):

```json
{
  "two_letter_iso_country_code": "AE",
  "latitude": 25.1164, "longitude": 56.3414,
  "city": "Fujairah", "region": "Fujairah",
  "org": "", "postal": "", "timezone": "Asia/Dubai",
  "asn": { "asn": "AS8849", "name": "Melbikomas UAB",
           "domain": "melbicom.net", "route": "89.36.162.0/24",
           "kind": "other" }
}
```

This settles the payload-width question by construction rather than by argument, and it makes the deferred migration's read path an identity mapping on the dVPN side. It also collapses what would otherwise be three value shapes into one, which is what makes the single-store layout in Decision 2 pay off: the entry value becomes `{ payload, checked_at, attestation: Option<{declared_at, signature}> }`, with `attestation` populated only for `Source::SelfDeclared`.

The canonical `Location` type lives in the contract's shared types crate, so contract, service and eventual HTTP consumers depend on one definition rather than converting between two. It sits behind a non-default `payload` feature, with the `utoipa` schema derive gated on top of that as the directory contract does.

**The `payload` feature gate is a wasm requirement, not tidiness.** CosmWasm rejects floating-point instructions at upload, `cosmwasm-check` enforces it over every artifact (`Makefile:197`, `.github/workflows/ci-contracts.yml:37`), and `Location` carries two `f64` coordinates. Whether the generated `f64` serde paths survive dead-code elimination is a link-time question, so relying on the contract simply not referencing the type would make blob validity depend on the build profile. A gated-off module is not compiled at all. The unification scope is the contracts workspace, since the release build is workspace-wide (`Makefile:97`), and that workspace holds only contracts; every payload consumer (nym-node, the geolocator service, node status API) lives in the root workspace and therefore cannot switch the feature on for the wasm build. The invariant to hold is narrow and greppable: no contracts-workspace member may take `payload` as a normal dependency. `cosmwasm-check` is the backstop that makes a violation loud rather than silent.

`serde_json`, which version 1 needs and which carries the `float_roundtrip` pin, is likewise an optional dependency under that feature, keeping it out of the contract's dependency tree entirely.

A separate leaf crate, mirroring `nym-directory-types`, was considered and rejected as disproportionate: the gate gives the same wasm guarantee for a payload surface amounting to `Location`, its ASN record, the version-1 codec, the typed signing payload and the node status API adapters. Pre-emptively homing the type in `nym-directory-types` itself was also considered and is blocked by sequencing rather than taste: that crate is not on develop, and this change merges before the branch that introduces it, so depending on it would invert the merge order. It is also prost throughout, where this payload is deliberately JSON, and a geolocation measurement is not a node-submitted directory entry. Whether the two payload homes should later be consolidated is a fair question to raise during the directory rebase, once both exist on one branch.

The one node status API field that cannot be reproduced is `geoip.ip_address`, a direct consequence of Decision 8. It will be sourced from the node's own announced addresses when present and left as the empty string otherwise, which is already that endpoint's no-data convention and which correctly leaves it empty for hostname-only operators.

The explorer and dVPN shapes still differ in encoding (stringified versus numeric coordinates, ASN present in one and absent in the other), so the canonical payload is the dVPN form and the follow-up adapts for the explorer surface at the HTTP boundary.

### 10a. The contract stores the payload opaquely, as versioned bytes

The contract never parses a location. The stored payload is `{ version: u8, content: Binary }`, exactly as the directory contract treats `NodeEntry.data`, and validation of its contents is the producer's and the consumer's job.

The decisive reason is signature integrity on the self-declared path. If the contract deserialised a node's signed location and re-serialised it for storage, the stored bytes could differ from the signed bytes through field ordering or float formatting, and the signature would no longer verify against what is on chain. Storing the signed bytes verbatim keeps the entry self-authenticating from current state alone, which is the property the whole directory design rests on.

The second reason is that it removes the payload encoding from the frozen surface. The digest leaf commits whatever bytes are stored, and a verifier recomputes without parsing them, so evolving the payload is a version bump rather than a contract migration and an accumulator re-fold. This is what downgrades the two `Location` encoding questions below from one-way doors to ordinary choices.

**Version 1 encodes `content` as JSON, not protobuf.** This diverges from `nym-directory-types`, whose payloads are prost messages, and it is a deliberate consumer-driven choice: the primary readers here are web applications, which get base64-decode followed by `JSON.parse` and never need the schema distributed to them. Obtaining and compiling a protobuf schema in a browser to read a country code is disproportionate.

That choice is what makes the `version` byte load-bearing rather than decorative. Prost's field tags handle additive evolution natively, which is why the directory contract needs no version field; JSON has no equivalent, so the byte becomes the actual evolution mechanism rather than cheap insurance. It sits outside `content` so a future version can change the *format*, not merely the schema.

`content` is `Binary` rather than a `String`. A `String` holding raw JSON would cost nothing extra in state and would even save consumers the base64 step, but it forecloses the non-JSON future the version byte exists for, and it double-escapes the payload in every query response. (A `String` holding *base64* would additionally pay roughly 33% inflation in state and in every leaf, since the compact codec stores whatever bytes the string contains.)

**JSON is not canonical, which makes verbatim handling a correctness invariant rather than an optimisation.** Prost output varies little between implementations, but JSON varies in key ordering, whitespace and float formatting. Because the self-declared path signs the payload bytes, any component performing `from_slice` followed by `to_vec` can emit different bytes and silently break verification. `Location` carries two `f64` coordinates, and without serde_json's `float_roundtrip` feature the default parser's fast path can round-trip a value such as `25.1164` to a different nearest-representable float. The workspace already pins that feature at `Cargo.toml:359` for exactly this reason, having hit it before on signed JSON payloads.

The invariant is therefore: the node signs the bytes it emits, the service relays them untouched, the contract stores them untouched, and any verifier checks the signature against the stored bytes rather than against anything it re-serialised. Nothing on the relay path may parse and re-emit a payload. This warrants an explicit test, not a comment.

What the contract gives up is the ability to reject a malformed payload, so a buggy whitelisted agent can write garbage that every consumer must tolerate. The directory contract already accepts this trade. What must not be given up is the size bound: without it a buggy agent can bloat state and inflate every verifier's recompute. The directory enforces this per label through `LabelConfig.max_size`; with closed enums here a single value suffices. That value lives in contract state rather than in a constant, because payload evolution is the whole point of the version byte and a later version may need more room, or less; making the bound admin-adjustable keeps that from being a redeploy. The constant is only what instantiation defaults to.

Uniformity of the payload across all sources (Decision 10) therefore becomes a convention owned by the shared types crate and the producers, not something the contract enforces.

Two encoding choices inside `Location` deviate deliberately from strict node status API parity. Both are cheap under Decision 10a, but neither is free to defer, so both are settled here.

**`asn` stores the provider's raw type, not the derived classification.** Node status API's `AsnKind` (`residential | other`, `http/models/mod.rs:186`) is derived by testing ipinfo's `type` for `"isp"` (`http/models/mod.rs:79`). Storing that derived form would permanently collapse `hosting`, `business` and `education` into `other`. The encoding is cheap to change later but the *data* is not: anything discarded at write time can only be recovered by re-measuring the whole network against a metered provider. Datacenter concentration is a decentralisation metric worth being able to ask about, so the raw type is stored verbatim and consumers derive the two-value form. The node status API adapter is a one-line match.

**Coordinates are `Option`.** Every other field in the payload has an unambiguous absent form: `asn` is already optional and the text fields use the empty string. Coordinates are the sole exception, because `0.0, 0.0` is a valid location off West Africa rather than a missing one. This is not hypothetical: nym-node currently carries only `location: Option<Country>`, so self-declared entries will essentially never have coordinates, and under a non-optional encoding every one of them would plot in the Gulf of Guinea. The adapter emits `0.0` for the node status API shape, preserving today's behaviour at the HTTP boundary while keeping absence explicit on chain.

Field parity is otherwise complete. `org`, `postal`, `city`, `region` and `timezone` are all retained: every one is already served publicly today by both the dVPN and explorer surfaces, so moving them on chain is a change of venue rather than new disclosure. The genuine delta is permanence, since the contract holds only current values but the transactions that wrote them persist in chain history, making a node's location changes permanently auditable in a way a rolling API snapshot is not. That was judged acceptable against the migration benefit, and narrowing later remains a payload version bump.

## Risks / Trade-offs

- **The key layout and leaf framing are frozen by the digest** → Decisions 2, 3 and 4 are one-way doors. Reversing any of them re-hashes every entry and requires a migration that re-folds the whole accumulator under a new `domain_tag`. They are settled before implementation for that reason. The payload *contents* are deliberately not in this category, because Decision 10a keeps them opaque to the contract and to the leaf.
- **An opaque payload means garbage can be committed and must be tolerated forever** → the contract cannot reject a malformed location, so every consumer parses defensively and a bad entry stays in the digest until overwritten or purged. Mitigated by producer-side validation in the shared types crate, and bounded by the max-size constant so the damage is a bad value rather than state bloat.
- **A JSON payload re-serialised anywhere on the relay path silently invalidates a node's signature** → JSON key ordering, whitespace and f64 formatting all vary between implementations, and `Location` carries two `f64` coordinates. Mitigated by treating verbatim handling as an invariant with a dedicated round-trip test, and by the workspace's existing `float_roundtrip` pin on serde_json (`Cargo.toml:359`). The failure mode is silent and delayed, so it must be caught by test rather than by review.
- **A mutation path that bypasses the digest wrapper breaks completeness permanently, and silently** → every state change routes through one wrapper, as the directory contract does; no handler touches the maps directly. Worth a test that fails if a new store method is added outside the wrapper.
- **A delete or update that subtracts bytes other than the exact old leaf corrupts the accumulator irrecoverably** → read the current value before removing or replacing it, and cover replace, delete and repeated-key-within-a-batch with tests.
- **Gas: batch size is guessed rather than measured** → `MAX_BATCH_SIZE` is set from a measured gas profile against the chain's per-transaction cap, with the accumulator load and save counted once per transaction.
- **Node clock ahead of chain time locks a node out of self-declaring** → bounded by `MAX_SKEW`, surfaced as a distinguishable error rather than a generic rejection.
- **Losing on-chain addresses means a restarted agent has no change-detection baseline** → first sweep after cold start establishes the baseline; changes during downtime are caught by the next regular sweep rather than by an explicit re-test.
- **`org`, `asn.name`, `city` and `postal` are permanently on-chain for every node** → for a self-hosted node this approaches the identifiability of the IP that Decision 8 protects, and unlike the IP it is immutable and indexed forever. The distinction drawn is that an IP is directly actionable and the rest is not, but the line deserves to be explicit rather than read later as an inconsistency. `postal` buys the least and costs the most; see Open Questions.
- **Deprecating the `SelfDeclared` kind later requires subtracting every affected leaf before deletion** → a delete-only migration leaves the digest permanently unverifiable. Recorded now so it is not discovered during the migration.
- **De-whitelisted agents' entries inflate the set every client recomputes over** → lazy paginated admin purge, on hygiene grounds only, since read-time authorisation already neutralises them.
- **Freshness drops from a 24h TTL to a monthly sweep on a user-visible surface** → this is the load-bearing justification for the explicit re-test path, which is why that endpoint is in scope rather than deferred with the migration.
- **The follow-up migration has a cold-start cliff**: `http/state.rs:431` drops any gateway whose country code is not exactly two characters, so switching against an unpopulated contract would empty the dVPN directory → the follow-up requires either a completed backfill sweep before cutover or the ipinfo path retained as a transitional fallback. Out of scope here, recorded so it is not a launch-day discovery.

## Migration Plan

This change is additive: a new contract and a new service, with no existing consumer switched over. Deployment is contract instantiation, agent whitelisting, then running the service against it. Rollback before any consumer depends on it is deleting the deployment; nothing else reads the contract yet.

The consumer cutover is the deferred follow-up change and carries the real migration risk, principally the cold-start cliff above and the removal of the required `IPINFO_API_TOKEN` configuration from node status API deployments.

## Open Questions

One item remains that requires work rather than a decision.

- **`MAX_BATCH_SIZE` (task 6.6)** must come from a measured gas profile against the chain's per-transaction cap, counting the accumulator load and save once per transaction. A batch of 50 to 100 is a hypothesis to test, not a value to adopt.

## Task 1 investigation findings (2026-08-05)

### 1.1 No custom `PrimaryKey` is needed

The premise of this spike was wrong in a useful direction. The directory contract never implements `PrimaryKey`: `StoredNodeEntries` uses the stock tuple impls, calls `key.key()`, and hands the parts to `Path::new` / `Prefix::new` itself (`contracts/directory/src/storage.rs:293`). So the question is not how costly a custom impl would be, but whether the key can be expressed with stock impls. It can:

```
  (u8 subject_class, Vec<u8> subject_id, Vec<u8> source_encoded)
```

a plain 3-tuple, with `Source` flattened to bytes by our own helper (`[1][method][agent]`, `[2]`, `[3]`) rather than being a key-component type. Section 3 gets simpler than planned.

**Ordering holds, and the reason is the per-class fixed-width rule.** cw-storage-plus length-prefixes every component except the last, so `Vec<u8>` components sort by length before content. That would break numeric node ordering if ids were variable-width. They are not: `NymNode` is always 4 bytes big-endian, so within a class the length prefix is constant and ordering falls through to the content. The fixed-width-per-class rule in Decision 2 was chosen for tidiness; it turns out to be load-bearing, which is why an id of the wrong width is rejected rather than accepted and left to sort oddly.

The trailing `source_encoded` is not length-prefixed, so it sorts lexicographically: `Measured` (tag 1) before `SelfDeclared` (2) before `Override` (3), and within `Measured` by method then agent. Correct without further work.

**One scan is not a prefix.** "All entries for a subject" is a prefix on `(class, id)`, mirroring `node_prefix`. "All measurements for a subject" is not, because `source_encoded` is a single opaque component and a prefix cannot reach inside it. This does not matter: a subject holds at most a handful of entries (one per agent, plus self-declared, plus override), so the per-subject scan filters in memory. Splitting `source` into two key components to make it a true prefix would add machinery for a scan of about five items.

**One assumption is compile-gated**: that cw-storage-plus 2.0 implements `KeyDeserialize` for `(u8, Vec<u8>, Vec<u8>)`. Both 3-tuples and `Vec<u8>` have it individually, so this is expected to hold, but it is the single thing a `cargo check` should confirm before section 3 commits to the layout.

### 1.2 `nym-directory-client` is reusable in layers, not wholesale

The earlier claim that it is "contract-agnostic, parameterised by contract address and digest key" was too strong. Measured against the branch:

| layer | coupling | reuse |
| --- | --- | --- |
| `proof.rs` | none: takes `(ops, app_hash, key, value)` | unchanged |
| `contract_storage_key` (in `nym-validator-client`) | none | unchanged |
| `anchor/checkpoint/*` | none | unchanged |
| `anchor/helpers.rs` | address-parameterised, but hardcodes the directory's `DIGEST_STATE` constant | needs the storage key threaded through as a parameter |
| `anchor/light_client.rs`, `anchor/proven.rs` | holds `directory_contract: AccountId` | small generalisation |
| `anchor/attested.rs`, `key.rs`, `client.rs` | directory-shaped throughout | sibling, not reuse |

So task 7.2 is "parameterise the digest key in `nym-directory-client`, then build a sibling client", not "reuse unchanged". The chain-level anchoring, which is the expensive part, genuinely does come for free.

### The finding that changed a requirement

`get_trusted_directory_digest` reads `DIGEST_LEN` bytes at the proven key and reconstructs via `LtHash16::from_bytes`. The as-built directory therefore stores the **full 2048-byte accumulator** at its raw provable key, and exposes the 32-byte collapse only through an unproven smart query. The spec here originally said the opposite, following the pattern note's "store the compact collapse for a small proof".

The as-built layout is better and the spec now matches it. The accumulator must be persisted on every mutation regardless, to support incremental updates, so persisting a collapse alongside it is an extra write per transaction for a value any client can compute. Proving the accumulator directly also removes the collapse from the set of things that must be computed identically on both sides. Had this gone unnoticed, the reused digest fetch would have rejected our contract on length.

## Resolved## Resolved

- **`NodeInformation.location`** is dropped during the directory rebase, not here. This change merges first and `feat/node-directory-publishing` is rebased on top of it, so the two-homes problem never materialises and nothing here depends on it.
- **Payload field parity** is kept in full. The fields are already public on today's node status API surfaces, so the disclosure delta is nil; the real delta is permanence, judged acceptable. Narrowing later is a version bump.
- **`asn` stores the raw provider type**, with the two-value classification derived at read time. Discarded data cannot be recovered without re-measuring the network.
- **Coordinates are `Option`**, with the adapter emitting `0.0` at the HTTP boundary.
- **`SubjectClass` is a closed enum**, consistent with `Method`. Adding a class is a code upgrade rather than an admin transaction, which suits a small, known, rarely-changing set.

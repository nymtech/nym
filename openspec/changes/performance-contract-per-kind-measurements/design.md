## Context

The contract on `develop` stores `results: Map<(EpochId, NodeId), NodeResults>` where `NodeResults(Vec<Percent>)` is every monitor's value for that node, kept sorted, with `median()` computed on read. Replay protection is a per-monitor cursor `NetworkMonitorSubmissionMetadata { last_submitted_epoch_id, last_submitted_node_id }` that requires each monitor to submit nodes in strictly ascending order within an epoch and never for an earlier epoch. Authorised and retired monitors are maps keyed by address. Every query is either a point load on `(epoch, node)` or a prefix range over an epoch; the node-history query walks epochs from the contract's creation epoch with a point load each, emitting `None` for gaps.

Its consumer today is nym-api's `ContractPerformanceProvider`, which caches whole epochs as `HashMap<NodeId, Performance>` and feeds one `RoutingScore` per node into rewarding, falling back to a previous *epoch's* score when the requested one is missing. nym-api's own scoring (`node_status_api/cache/refresher.rs`, `PerformanceComponents::performance`) is a renormalised weighted mean of whichever routing properties applied, multiplied by the config score, and `None` when nothing applied. That formula is reward-affecting and this contract mirrors it, so that the first iteration (nym-api relays the contract's score) and the second (the mixnet contract queries it directly) cannot diverge.

The producer will be network-monitor-v3's orchestrator, which materialises per-`(node, epoch, kind)` aggregates at each epoch's start (the window preceding the epoch), keeps liveness and stress today, and computes the config score as a third aggregate. It backfills epochs missed while it was down. It does not yet submit to this contract; that is a follow-up.

Three facts settled during design shape the layout below. A `node_id` is a bonded lifetime: it is never reused after unbond, so a node's history is contiguous and the only gaps in it are monitor outages. Kinds are role-dependent: a mixnode will never carry a dvpn or gateway measurement, so a node that switches role legitimately stops having a kind. And the contract is not deployed anywhere, so nothing here is a migration.

## Goals / Non-Goals

**Goals:**

- Store per-kind, per-monitor values so a consumer can compute the final figure and a user can see the breakdown.
- Make "what values were used for rewarding epoch X for node Y" answerable at any later time, from the contract alone, with no recording at use-time.
- Fall back cheaply to the last epoch with data when an epoch is missing, with a bounded, predictable read cost that the mixnet contract can afford per node per epoch.
- Keep the score formula identical to nym-api's, including its refusal to invent a score when nothing applied.
- Keep storage proportional to what is kept: no per-kind key repetition, no index that only pays for itself in a case that cannot occur.
- Keep the kind set closed and typed, with a new kind an additive code change that needs no stored-data migration.

**Non-Goals:**

- **The nym-api consumer.** A follow-up change decides how nym-api adopts `RewardingInputs`. This change edits nym-api only enough to compile.
- **The submission path.** A follow-up in network-monitor-v3. This design fixes what it must send (the current epoch only, per-node bundles sorted by node id) but does not implement it.
- **Recording what the mixnet contract used.** Determinism over frozen data makes that unnecessary; see Decision 5.
- **Compaction of closed epochs to medians.** It needs a separate process and discards the per-monitor values that let a misbehaving monitor be audited. At the scale in Decision 10 the raw data is affordable.
- **Any role model in the contract.** Renormalisation over what applied handles role differences without the contract knowing roles.

## Decisions

### 1. Every kind is a `Percent`, aggregated by median; the kind set is fixed and named

Every measurement kind is submitted as a single `Percent` per monitor and aggregated by median across monitors, exactly as the single value is today, split by kind. The orchestrator's own aggregates carry richer shapes (a score plus a sample count for liveness and stress), but what reaches the chain is the score, and the count is evidence for the orchestrator's API rather than an input to rewarding. Keeping the on-chain value uniform keeps one aggregator, one storage shape and one query shape.

The kinds are a fixed set expressed as named optional fields, `liveness`, `stress` and `config`, on each per-kind type: `Measurements` for a submission's values, `EpochNodeMeasurements` for the stored bundle, `KindMedians` for medians. The stored and submitted shapes carry `#[serde(rename)]` to one character each (`l`, `s`, `c`), because they are JSON and the kind name is paid once per kind per bundle, and an absent kind is omitted from the JSON entirely. There is no kind enum. An earlier draft keyed `BTreeMap`s by one, and the contract's JSON codec (`serde-json-wasm`) panics when deserialising an enum used as a map key; that forced named fields, and once the fields existed the enum had no remaining use, since the split the score formula depends on is by field: `liveness` and `stress` are routing kinds and carry weights; `config` is the multiplier and has no weight field, so it cannot be weighted by construction. A new kind is a new optional field on each per-kind type, and because a missing field reads back as `None`, existing entries need no migration and a monitor built before the field existed simply never sends it. Adding a field is a coordinated, contract-first deploy, since an older contract rejects an unknown field.

PR #6282's `MeasurementKind = String` with an admin-defined registry was rejected: the string is repeated in every composite key, an open set forfeits exhaustive matching and the typed routing/multiplier split, and a typo silently creates an unreachable kind. The registry itself is dropped too (Decision 9).

### 2. One bundle per `(epoch, node)`, holding every kind

```
  performance_results  (epoch_id, node_id) -> EpochNodeMeasurements
                                             { liveness: Option<NodeResults>, stress: Option<NodeResults>, config: Option<NodeResults> }

  stored JSON, 3 monitors, 3 kinds:
  {"l":["0.93","0.95","0.97"],"s":["0.8","0.8","0.85"],"c":["1","1","1"]}
```

`NodeResults` is unchanged: sorted on insert, never empty, `median()` on read. A monitor's submission for a node is merged field by field: an existing `NodeResults` takes `insert_new`, an absent one is created from the single value, which preserves the never-empty invariant without giving `NodeResults` a `Default`. The median for a kind is over exactly the monitors that reported that kind for that node: a monitor that did not measure config contributes nothing to config rather than a zero, which is the "absent, not zero" rule the orchestrator spec already commits to.

Three layouts were rejected. Per-`(node, kind)` rows keyed `(epoch, node, kind)`, as #6282 did, multiply the key count by the number of kinds and pay the composite key's namespace and length prefixes per kind, and force the replay cursor to grow a kind dimension. Kind-major rows `(kind, epoch, node)` optimise a network-wide scan of one kind, which nothing needs, at the cost of the per-node combine that everything needs. An `IndexedMap` with a secondary index on `node_id` was adopted and then dropped once it was established that a `node_id` is never reused: its remaining benefits were skipping gaps in sparse histories, which do not exist, and bounding the history walk, which the pointer in Decision 6 does for free. It would have cost about 25 bytes per bundle and made purge go through the index API.

Epoch-major keys keep whole-epoch pulls as native prefix ranges, which is the first iteration's hot path, and keep `RemoveEpochMeasurements` a prefix clear.

### 3. A submission is per node with any subset of kinds, and the replay cursor is unchanged

```
  NodeSubmission { node_id ("n"), measurements ("m"): Measurements { liveness?, stress?, config? } }

  ExecuteMsg::Submit      { epoch, data: NodeSubmission }
  ExecuteMsg::BatchSubmit { epoch, data: Vec<NodeSubmission> }   -- strictly ascending node_id
```

Because a node is submitted once per epoch with all of its kinds together, the existing per-monitor cursor `(last_epoch_id, last_node_id)` still guarantees that a monitor contributes at most one value per `(node, kind)` per epoch. This is what resolves the staleness problem #6282 left open: with per-`(node, kind)` submissions, `(node1, liveness)` followed by `(node1, config)` fails the `node_id > last` check, and the cursor would have needed a kind dimension. One field per kind also means a monitor cannot send the same kind twice for one node. A `measurements` with every kind absent is rejected outright, since it would create or touch a bundle with nothing and advance the cursor for no reason.

The orchestrator materialises every kind for every node at once, so a per-node bundle is its natural unit. The whole epoch is still chunked into batches for gas and message size, exactly as today.

### 4. Only the current mixnet epoch is writable

A submission is accepted only when `epoch == current mixnet epoch`, queried from the mixnet contract as the cursor initialisation already does. Once the mixnet advances, a bundle can never change again.

This is what makes Decision 5 possible. Rewarding for epoch X runs after the advance to X+1, so what it reads is final; a monitor recovering from an outage cannot land a late value into X and change the median after it was used, which is precisely the outage case where an audit matters. It also subsumes rejecting future epochs, which nothing did before.

The cost is explicit: the orchestrator's backfill of missed epochs never reaches the chain. A backfilled X would arrive after rewarding for X and could never have been used, so putting it on-chain would only make the record disagree with what rewarding saw. Backfill stays valuable for the orchestrator's own API. Two minor consequences: a monitor must submit promptly after materialising, which at hourly epochs is not a constraint, and batch chunks that straddle an epoch boundary are rejected, leaving those nodes' medians over fewer monitors, which is the truth of what arrived in time. Running several monitor instances lessens the missing-data cost and strengthens the median at the same time.

### 5. Fallback is per bundle, deterministic, and is the audit query

"What values were used for rewarding X for Y" can be answered by recording the resolution at use-time or by making it a deterministic function of immutable data. Recording is out for this contract: a query cannot write, and the rewarder writing back per node is a thousand transactions an epoch. So the resolution is a pure function of frozen bundles.

```
  resolve(X, node):
      if bundle(X, node) exists            -> (X, bundle)
      p = last_known_epoch[node]            -> None if absent
      start = min(p, X - 1)
      for e in start down to X - L          -> first existing bundle(e, node)
      None
```

`L = MAX_FALLBACK_LOOKBACK_EPOCHS = 24`, a contract constant rather than a caller parameter, because two callers passing two values would get two answers for one epoch. It is small because lifetimes are contiguous: the walk length is the outage length, and a node with no data for a day is unmeasured rather than stale. The pointer is a short-circuit, not a source of truth: when `p <= X - 1` the first load usually hits, and when bundles exist after X the walk starts at X-1 regardless. A dangling pointer after an admin purge simply walks.

**Fallback is per bundle, never per kind.** An earlier draft resolved each kind independently to its latest epoch. That reads a role switch as an outage: a node that was a gateway at X-3 and is a mixnode at X would have its old dvpn score folded into the mean at X, and no cheap signal distinguishes "missing because a monitor was down" from "missing because the kind no longer applies". Per bundle, a kind absent from an epoch in which the node *was* measured means "not measured this epoch", which is the correct reading because at least one monitor reached the node and its kind set reflects what applied then. The cost is that a partial-kind outage drops that kind for the epoch instead of borrowing it; the node is not penalised, renormalisation scores it on what is present, and it only reaches "no score" if config is present with no routing kind at all, which needs every liveness-producing instance down at once, the full-outage condition. This is also what nym-api's contract provider already does: it falls back to a previous epoch's whole score, never per component.

Because the rule is deterministic over frozen data, `RewardingInputs { epoch_id: X, node_id: Y }` returns identical answers whenever it is called. It is therefore the rewarding query in both iterations and the audit query, with nothing recorded.

It has a score-only projection, `RewardingScore { epoch_id, node_id } -> { score: Option<Percent> }`, for a consumer that needs only the value: the mixnet contract in the second iteration reads that, while `RewardingInputs` carries the medians, source epoch and weights for anyone auditing the number. The two are one resolution function with two response shapes, and the projection is never allowed its own code path, so they cannot drift.

### 6. A per-node last-known-epoch pointer

`last_known_epoch: Map<NodeId, EpochId>`, written when a bundle for `(epoch, node)` is first created (the first monitor to report that node in that epoch), as a max. Under Decision 4 the new epoch is always the current one, so the max is a guard rather than a branch that fires. It costs one write per node per epoch and O(nodes) storage, flat over time.

It serves three things: the "last epoch where data was available" query the consumer asked for, directly; the short-circuit in Decision 5; and the upper bound of the node-history walk, which otherwise emits `None` up to the current epoch for a node that unbonded long ago. A per-`(node, kind)` pointer was considered when fallback was still per kind and became pointless with Decision 5.

### 7. Weights live on-chain in a sparse epoch-keyed map

```
  weights  (effective_from: EpochId) -> Weights { liveness: Percent, stress: Percent }

  instantiate     -> weights[creation_epoch] = initial_weights
  UpdateWeights   -> weights[current_epoch + 1] = new_weights          (admin only)
  weights_at(X)   -> descending range bounded at X, take 1
```

If "what was used for rewarding" is to include the combined score, the weights must be on-chain and versioned by epoch; otherwise the breakdown is reconstructable but the combination is not. The map is written only on change, so it holds a handful of entries over the contract's lifetime, and "weights at X" is one bounded range. Storing under `current + 1` means an update made mid-epoch applies from the next epoch, so every epoch's weights were fixed before that epoch began and an operator gets "changing weights affects the next epoch onward" as a clean invariant. Two updates within one epoch land on the same key and the later overwrites, which is the right outcome. The initial weights go under the creation epoch itself so that no epoch with data is ever without weights.

A `SnapshotMap` was the first thought and does not fit: it is keyed by block height, so "weights at epoch X" would first need epoch to height, which nothing stores, since the mixnet tracks epochs by time; its changelog strategies exist for values that churn; and with one logical value it degenerates to an `Item` with history, which the sparse map already is. Holding the weights in the consumer was rejected because it puts the one non-reconstructable input of the audit off-chain and lets the two iterations diverge.

### 8. The score mirrors nym-api's formula, in `Decimal`

```
  applied = routing kinds with a non-zero weight that are present in the resolved medians
  score   = ( sum(w_k * median_k) / sum(w_k) ) over applied, times median_config
          = None  if applied is empty, or config is absent
  rounded to two decimal places at the end
```

This is `PerformanceComponents::performance` in nym-api: a renormalised weighted mean of whichever routing properties applied, multiplied by the config score, and `None` when nothing applied because inventing a value either way is wrong. Renormalising over what applied is what lets one weight set serve every role; a mixnode's stress share and a gateway's lack of one are both handled by the mean. A kind present in the bundle whose weight is zero is not applied, which is what makes collecting a new kind unweighted safe. Config is always produced for every node, so a missing config is a defect to surface as "no score", not a case to paper over with a multiplier of one.

The contract necessarily differs from nym-api in ways that do not change the result. The arithmetic is `Decimal`, since wasm cannot use floats, which also removes nym-api's tolerance dance: `0.7 + 0.2 + 0.1` is exactly `1` in `Decimal`, so the sum check is exact. `Weights` is a struct with one `Percent` field per routing kind, `liveness` and `stress`, each `#[serde(default)]` so that a field absent from an older message reads as zero, and zero means the kind does not contribute. On-chain there is no separate `enabled` knob for a zero to conflict with, so nym-api's zero-is-ambiguous argument does not apply here, and config cannot be weighted because it has no field. Named fields rather than a map because the set is fixed and small and a field per member reads and evolves better than a composite-key map. Validation on `UpdateWeights` and at instantiation: at least one non-zero weight, and a sum of exactly one; each weight is in `[0, 1]` by the `Percent` type. nym-api's "routing or liveness must be enabled" rule encodes role knowledge the contract does not have, and its honest analogue is already present: an empty applied set yields no score, visibly. nym-api's rule that no scoring property may be enabled alongside `use_performance_contract_data` belongs to the nym-api follow-up.

Every medians view returns two separate things side by side: the per-kind medians, each taken across the monitors that reported that kind and never weighted, and a `score` field, which combines those medians across kinds under the weights in force at *that* epoch. Weights apply across kinds only; one monitor's value is never weighted against another's. `RewardingInputs` is the one place the two epochs can differ: its medians come from the resolved (possibly earlier) bundle, and its score is computed with the weights in force at the *requested* epoch, because that is what rewarding for the requested epoch used. The response carries both epochs so the difference is visible.

### 9. No runtime kind registry

With a closed enum, #6282's `DefineMeasurementKind` / `RetireMeasurementKind` offers only a runtime gate on acceptance, at the cost of a map, a check on every submission and response filtering to keep audited. It is dropped. Any variant is accepted and stored; the weights map decides what scores. That gives a better staged rollout than a define gate, since a new kind can be collected unweighted and inspected before it is given a share. Retiring a kind is removing its weight, which stops it affecting scores immediately; if its storage ever matters, an upgrade rejects the variant.

### 10. The storage budget, and what is and is not worth optimising

At 1000 nodes, hourly epochs, three kinds and three monitors: 24,000 bundles a day. Encoded as JSON with quoted two-decimal `Percent` strings a bundle is about 71 bytes of value plus about 14 bytes of key with a two-character namespace, so roughly 2 MB a day and 0.75 GB a year of raw key-value data; with the integer encoding below it is about 63 bytes a bundle, 1.5 MB a day and 0.55 GB a year. That is before the chain's own tree overhead, which is on the order of 150 to 200 bytes per entry regardless of value size, so once values are small the entry count dominates physical storage rather than the bytes per value. That lever was already pulled by Decision 2: one bundle per node instead of one row per kind cut the entry count threefold, and dropping the index halved it again. The value scales with kinds times monitors; key layout is a rounding error, which is why Decision 2 was made on query shape rather than bytes.

Three cheap choices are taken. The bundle map's namespace is shortened to `pr`, because it is the only namespace paid once per `(epoch, node)`; every other map's cardinality is monitors, nodes or weight changes, and those keep descriptive names. The stored types use one-character serde renames, continuing the `"n"`/`"p"` convention already in `NodePerformance`. And each stored value is a `u8` integer percent rather than a `Percent` decimal string: the two-decimal rounding already collapses the domain to the 101 values `0..=100`, so `93` replaces `"0.93"` with nothing lost, taking a kind's three values from `["0.93","0.95","0.97"]` to `[93,95,97]`. This is a storage representation inside `NodeResults`, converting with `round_to_two_decimal_places().round_to_integer()` on insert and `Percent::from_percentage_value` on read, so `Percent` stays the type at every boundary and no message or shared type changes; the raw bundle view naturally shows the integers.

Two larger levers are declined. Compacting closed epochs to per-kind medians would cut the value threefold but needs a separate process and discards the per-monitor evidence that makes a bad monitor auditable. A hand-rolled byte codec in place of JSON would take the value from about 49 bytes to about 16, but it costs bespoke framing and a version byte to migrate, storage plumbing outside `cw-storage-plus`'s `Map` in the directory-contract style for every paged query, serde's free additive evolution, and a raw state nobody can read without the decoder. That last cost lands on the audit requirement: only raw store reads carry ICS23 proofs, so a proven audit read should be self-describing. The geolocation change measured this same trade (473 against 304 bytes, its Decision 10b) and declined it for the same reasons, and the tree overhead above caps what it could buy physically at roughly a seventh. It is not a one-way door: nothing here commits the encoding to a digest, so a codec can be introduced later by a paged re-encoding migration if storage ever binds. Retention remains the admin purge that already exists.

## Risks / Trade-offs

- **An admin purge destroys reconstructability for that epoch** → `RemoveEpochMeasurements` and `RemoveNodeMeasurements` are documented as an intentional loss of the audit trail, intended for epochs older than any consumer cares about. The pointer is not touched by them and the fallback walk tolerates a dangling one.
- **Changing `L` is a contract upgrade that shifts past audits** → recorded so it is not done casually. A change is visible on-chain as an upgrade, and "the audit query as the contract computes it today" is the honest statement of what it returns.
- **A partial-kind outage drops evidence rather than borrowing it** → accepted in exchange for never mis-scoring a role switch; multiple monitor instances make it rare.
- **The chain never holds backfilled epochs** → accepted for the same auditability; the orchestrator keeps its backfill for its own API.
- **Boundary-straddling batch chunks are rejected** → the affected nodes' medians are over fewer monitors for that epoch. Submitting promptly after materialisation makes this rare at hourly epochs.
- **`Decimal` versus `f64` rounding differs from nym-api in the last digits** → both round the published figure to two decimal places, and the first-iteration follow-up relays the contract's score rather than recomputing, so no comparison is made at full precision.
- **The first-iteration nym-api must read after the epoch-advance transaction commits** → otherwise a last-second submission can slip between its read and the freeze. Belongs to the follow-up; recorded so it is designed in rather than discovered.
- **Adding a kind is a coordinated, contract-first deploy** → an older contract rejects an unknown variant, so the contract upgrades before any monitor sends the new kind.
- **Weighted kinds that no node of some role can have** → not a failure: renormalisation drops them for that role. What would be a failure is a weight set whose only kinds apply to no role at all, which surfaces as every node of that role having no score, visibly, rather than being prevented by a role rule the contract cannot express.

## Migration Plan

Nothing is deployed, so there is no state migration. Deployment is instantiating the contract with the mixnet contract address, the initial monitors and the initial weights, then authorising monitors. `tools/internal/localnet-orchestrator` supplies a valid default weight set so localnet keeps working. PR #6282 is closed in favour of this change.

The follow-ups sequence after this merges: the network-monitor-v3 submission path (which also brings the orchestrator's config-score aggregate into the same submission), then the nym-api consumer for the first iteration, then the mixnet contract's direct query for the second. This change merges first and the network-monitor-v3 work is rebased on top of it.

## Open Questions

None outstanding. `L` is set at 24 epochs (one day); 48 was the alternative and it is a one-constant change if operational experience wants it. `Stress` is the kind's name today because that is what network-monitor-v3 produces; a dvpn measurement, when it exists, is an additional variant rather than a rename.

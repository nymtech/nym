## RENAMED Requirements

- FROM: `### Requirement: nym-api reads the performance contract only behind an explicit flag, which the contract-mode switch requires`
- TO: `### Requirement: Every nym-api reads the performance contract, and only the rewarding one holds the window`

## MODIFIED Requirements

### Requirement: Every nym-api reads the performance contract, and only the rewarding one holds the window

Every nym-api SHALL read the performance contract, producing attested performance snapshots as `performance-attestation-provider` specifies, whatever its configuration. No setting SHALL decide whether nym-api reads the contract, and it MUST NOT be inferred from the network details carrying the address: the address is assumed present, and network details that do not carry it MUST fail startup. The window of the contract's per-epoch medians ("nym-api keeps a window of the performance contract's per-epoch medians") SHALL be held exactly when `[rewarding].enabled` is set, the only nym-api on which its readers exist: contract-mode rewarding and active-set selection and, with the switch off, the `contract-divergence` route. `[performance_provider].use_performance_contract_data`, the switch, defaulting to `false`, SHALL decide whether nym-api uses its own figure or the contract's: with it on, rewarding and active-set selection use the contract's figure on the nym-api that rewards, every nym-api publishes the contract's verified routing figure, and nym-api computes no figure of its own, as the consumer-surface requirement specifies. The switch MUST NOT require any other setting. Enabled `[performance_provider.scoring.*]` properties MUST NOT be rejected alongside the switch: they have no effect in contract mode, as the consumer-surface requirement specifies.

Startup MUST depend on the mode on the nym-api holding the window. With the switch off, the window is loaded best effort: a failure to load it is logged, the window keeps retrying at its polling interval, and nothing else waits for it. With the switch on, the window MUST be loaded at startup, before the epoch operations start, and startup MUST fail when it cannot be, or when the contract has never received a submission. Contract-mode rewarding and selection have no other figure: an empty contract would pay every node zero and, since a node without a figure is skipped, leave the active set empty, whereas a contract that holds data can never become empty again, its bundles being append-only. A nym-api without `[rewarding].enabled` holds no window and MUST NOT load or check one at startup.

`[performance_provider].read_performance_contract`, `[performance_provider.debug].max_performance_fallback_epochs` and `max_epoch_entries_to_retain` SHALL no longer exist. A configuration that still sets any of them MUST load, the value being ignored.

#### Scenario: A nym-api that does not reward holds no window
- **WHEN** nym-api runs without `[rewarding].enabled`
- **THEN** it produces performance snapshots, holds no window, reads no per-epoch medians for one, and its `contract-divergence` route answers 503

#### Scenario: The switch needs no other setting
- **WHEN** a configuration sets `use_performance_contract_data` and nothing else in `[performance_provider]`
- **THEN** validation accepts it

#### Scenario: A missing address fails startup
- **WHEN** the network details carry no performance contract address
- **THEN** startup fails, whatever the configuration

#### Scenario: Reading is best effort while the switch is off
- **WHEN** `[rewarding].enabled` is set, the switch is off, and the window's first load fails
- **THEN** nym-api starts, the failure is logged, the load is retried at the polling interval, and the `contract-divergence` route answers 503 until it succeeds

#### Scenario: A contract that never received a submission stops a contract-mode start
- **WHEN** `[rewarding].enabled` and the switch are both on, and the contract's last submission carries no data
- **THEN** startup fails before the epoch operations start

#### Scenario: The scoring section stays as it is across the flip
- **WHEN** the switch is turned on in a configuration whose scoring section enables `legacy_v1_routing` at its default weight
- **THEN** validation accepts it, and turning the switch off again restores scoring under exactly those weights

#### Scenario: A removed knob does not break a configuration
- **WHEN** a configuration still sets `read_performance_contract`, `max_performance_fallback_epochs` or `max_epoch_entries_to_retain`
- **THEN** it loads and the value has no effect

### Requirement: nym-api keeps a window of the performance contract's per-epoch medians

A nym-api with `[rewarding].enabled` SHALL keep a local window of the performance contract: for each epoch from `R - MAX_FALLBACK_LOOKBACK_EPOCHS` (24) to R, and for the current mixnet epoch X, every node's per-kind medians as `EpochPerformancePaged` returns them, together with the weights in force at X (`WeightsAt { epoch_id: X }`). R SHALL be the newest held epoch not after X in which any node has a bundle. The window MUST NOT keep the pages' scores, which are under each epoch's own weights rather than X's, and MUST hold an epoch read with no nodes as absent, so that it never counts as R.

At startup the window MUST read `LastSubmittedMeasurement` and the weights at X, and, when the contract has received a submission, page the newest submission's epoch, even when X is lower because the mixnet contract cache is behind, and each earlier epoch until one at or below X holds nodes, which is R, then every epoch down to `R - 24`. Submissions go only to the current epoch, so no bundle lies above the newest submission's epoch and no epoch above it is paged. That epoch is paged even above X because X can later reach it without a new submission to trigger a re-page. No epoch below `R - 24` is ever needed, however long an outage lasted, because the contract anchors its fallback at R. When none of the 25 epochs from where the walk starts holds nodes, the load MUST fail rather than walk on toward epoch 0. When the contract has never received a submission, the window holds nothing. Every `contract_polling_interval` (default 150 seconds) the window MUST read `LastSubmittedMeasurement` and the current epoch. When the current epoch has moved on, it MUST read the weights at the new X. When the last submission differs from the one last seen, it MUST re-page every epoch from the recorded submission's epoch to the new one's: only the current epoch takes submissions, but it can move on between two polls, so each epoch that was current at some point in between may have received one. A window that has seen no submission yet MUST be loaded afresh instead. It MUST then drop every epoch below `R - 24`. A poll MUST read everything it needs before changing the window, so that a failed poll leaves it as it was. The current epoch the window holds MUST never move backwards: a poll naming an older epoch keeps the one held, so that a caller with a stale view cannot move the figures, and with them the rewards being computed, to the previous epoch. The last submission the window records MUST be the one read before the pages it triggered, so that a submission landing while they are read shows up as a change at the next poll rather than being masked.

The window is exact against the contract because only the current epoch is writable: every epoch below X is frozen once paged, and the epoch operations run one more poll once X is sealed. Its pages therefore need not be pinned to one height: a node written between two pages of X is picked up when the next poll sees the last submission change.

#### Scenario: A start after an outage loads only what the rule can reach
- **WHEN** nym-api starts while the newest submission is for epoch 100, every epoch from 60 to 100 holds bundles, and the current epoch is 130
- **THEN** the window holds epochs 76 to 100 and the weights at 130, pages no epoch above 100, and holds epoch 130 as absent

#### Scenario: A newest submission whose epoch holds nothing
- **WHEN** the newest submission's epoch holds no bundle, because every node of its batch was skipped as not bonded, and the newest epoch holding bundles is two below it
- **THEN** the load walks down to that epoch as R and holds the 24 epochs below it as well

#### Scenario: A start while the mixnet contract cache is an epoch behind
- **WHEN** nym-api starts while the newest submission is for epoch 100, which holds bundles, and its mixnet contract cache still names epoch 99
- **THEN** the window pages epoch 100 too, and once the cache names 100 it resolves every node from epoch 100's bundles without waiting for another submission

#### Scenario: A load finding nothing near the newest submission fails
- **WHEN** the contract has received a submission but none of the 25 epochs from where the walk starts holds a bundle
- **THEN** the load fails after paging those 25 epochs, and pages nothing below them

#### Scenario: A new submission is picked up at the next poll
- **WHEN** a poll finds that the last submission has changed and names the current epoch X
- **THEN** X is re-paged, and it becomes R with every epoch below `X - 24` dropped

#### Scenario: A new epoch keeps the window while R stays
- **WHEN** the current epoch advances from X to X + 1 while nothing has yet been submitted for X + 1
- **THEN** the weights at X + 1 are read, no epoch is paged, X + 1 is held as absent, and no epoch from `X - 24` to X is dropped

#### Scenario: A stale caller does not move the window back
- **WHEN** the window holds X and a poll names X - 1
- **THEN** the window keeps X and its weights, and reads nothing but the last submission

#### Scenario: A late chunk is not lost to the next epoch's first
- **WHEN** between two polls a chunk lands for X, the epoch moves on to X + 1, and a chunk lands for X + 1
- **THEN** the next poll re-pages both X and X + 1, and the window holds both chunks

### Requirement: nym-api serves a temporary contract-divergence route while it holds both figures

On the nym-api holding the window, the one with `[rewarding].enabled`, with the switch off, nym-api SHALL serve `GET /v3/unstable/nym-nodes/contract-divergence`, comparing, for every bonded node, N, the `performance_score` of nym-api's annotation, with C, the node's contract figure at the current epoch X. Each node MUST land in exactly one category, the first that matches: `missing` (N > 0 and C has no source), `contract_only` (no annotation and C has a score), `unscoreable` (C has a source but no score), `fallback` (C's source epoch is below X), `divergent` (|N - C| at least the threshold), and `agree`. A node that has no annotation or N = 0, and no source in C, MUST NOT be listed or counted. The threshold SHALL be a fixed 0.2, not a request parameter. `fallback` precedes `divergent` so that a stale contract figure is never reported as a measurement disagreement.

The response MUST carry X, the threshold, `refreshed_at` (the oldest of the annotation cache, the describe cache and the window), and for each category a count in total and per declared role (mixnode, entry and exit, from nym-api's describe cache), a node counting under every role it declares and a node nym-api has not described counting in the total only. It MUST carry one row per node in every category except `agree`, holding: the category; nym-api's N, routing score and config score when the node has an annotation; C's source epoch, score, and liveness, stress and config medians when C has a source; the node's role in the current rewarded set, if it has one; and, on `missing` rows only, the node's version and declared roles.

The route MUST stay mounted in every mode. On a nym-api holding no window, with the switch on, or before the window has loaded, it MUST answer 503 with a message naming what is missing: the contract side (this nym-api does not reward, so it holds no window), nym-api's own figure (which contract mode does not compute), or the window. It is TEMPORARY: it exists for the shadow period before the switch is turned on, it is served under the unstable route tree, and it is to be deleted afterwards, so nothing durable may be built on it.

#### Scenario: A node the contract has nothing for is missing
- **WHEN** a node's N is 0.9 and C has no source
- **THEN** it is `missing`, and its row carries nym-api's figures, its version and its declared roles

#### Scenario: A node only the contract scores is contract-only
- **WHEN** a node has no annotation and C has a score of 0.7
- **THEN** it is `contract_only`, and its row carries the contract's side only

#### Scenario: A source the contract cannot score is unscoreable
- **WHEN** C has a source without a score, whatever N is
- **THEN** the node is `unscoreable`

#### Scenario: A stale contract figure is fallback, not divergent
- **WHEN** a node's N is 0.9 and C is 0.2 from a source epoch below X
- **THEN** it is `fallback`, and its row carries both figures

#### Scenario: A measured disagreement is divergent
- **WHEN** a node in the current rewarded set has N 0.9 and C 0.6 from a bundle at X
- **THEN** it is `divergent`, and its row carries both sides and its rewarded-set role

#### Scenario: Both sides scoring zero agree
- **WHEN** a node's N is 0 and C is 0 from a bundle at X
- **THEN** it is `agree`, counted, and has no row

#### Scenario: A node neither side has anything for is not listed
- **WHEN** a bonded node has no annotation and C has no source
- **THEN** it is neither counted nor listed

#### Scenario: Counts are per declared role
- **WHEN** a node declaring both the mixnode and the entry role is `divergent`, and a node nym-api has not described is `contract_only`
- **THEN** the `divergent` count includes it under both mixnode and entry, and the `contract_only` count includes the undescribed node in its total and under no role

#### Scenario: A difference below the threshold agrees
- **WHEN** a node's N is 0.9 and C is 0.75 from a bundle at X
- **THEN** it is `agree`, the difference being below 0.2

#### Scenario: The route names the side it lacks
- **WHEN** the route is called on a nym-api that does not reward, with the switch on, or before the window has loaded
- **THEN** it answers 503 with a message naming respectively the contract side, nym-api's own figure, or the window

### Requirement: Stored stress-test scores feed node performance and rewarding through a defined consumer surface

The stored stress-test and liveness results SHALL form the subsystem's output contract to the rest of nym-api; the detailed behaviour of each consumer is owned by its own capability, and this requirement fixes only WHICH subsystems read the results and FOR WHAT. The stored per-node stress results MUST be aggregated (average performance and a reachability flag over a configured window) into a stress-testing score; the stored per-node liveness results MUST be aggregated the same way into a separate liveness score; each score MUST feed the node performance provider, which folds them - together with routing and configuration components - into each node's detailed performance, each gated by its own `use_*_data`, `minimum_available_*_results`, and `*_score_weight` configuration flags; and in legacy mode the resulting composite performance MUST flow into rewarding and active-set selection via the node's rewarding-performance derivation.

Everything this requirement describes is nym-api's OWN figure, and it is computed in legacy mode only, that is while `use_performance_contract_data` is off. With it on, nym-api MUST compute no figure of its own: the node-status refresher MUST skip the provider reads, the availability thresholds, the config score and the carry-over below, and MUST instead build the annotation map from the performance contract's verified routing figure, as "In contract mode nym-api publishes the verified routing figure as each node's performance" specifies. That map MUST replace the previous one rather than be merged into it, because the annotation cache is persisted, so a legacy map loaded from disk would otherwise go on being served after the switch is turned on. The `[performance_provider.scoring]` section MUST still be validated by the rules below and MUST have no effect while the switch is on, so that turning the switch on or off is a single change and leaves the weights in force before it untouched. In that mode rewarding and active-set selection take each node's figure from the performance contract, as "In contract mode rewarding pays the contract's figure for the sealed epoch" and "In contract mode the active set is selected by the contract's figure, and by stake alone when nothing was submitted at the epoch" specify.

Liveness MUST be INERT by default, so that it is recorded and queryable without affecting performance or rewarding until an operator deliberately enables it. This is required because two populations will score zero on liveness for reasons unrelated to their forwarding capability: nodes that have not ingested their agents' on-chain authorisations, and gateways not yet carrying the final-hop and monitor-session behaviour that gateway liveness depends on.

Node performance SHALL be computed as the weighted mean of whichever delivery properties APPLIED to that node, renormalised over their weights, MULTIPLIED by the node's config score. The properties are the legacy v1 routing score, the v3 stress score and the v3 liveness score; they measure the same thing from different sources, so their weights are proportions of one figure rather than independent axes.

This supersedes a formula in which each enabled component took its weight and `routing x config` absorbed the remainder. That form let a component ESCAPE the config gate: with stress at 0.3, three tenths of a node's performance was immune to its configuration score, so a node that was out of date, unfunded or had not accepted the terms could partially buy that back by stress-testing well. Multiplying the whole delivery figure by the config score removes that shield, and the difference for a given node is exactly `stress_weight x stress_score x (1 - config_score)` - zero for a node whose config score is 1.0.

Renormalisation over the APPLIED set is required rather than optional, because that set varies for two reasons outside the operator's control. A property may not apply to a node at all - stress testing covers mixnodes only, so a gateway would otherwise be docked the stress share for a test it cannot take - and a property may be dropped mid-flight when its availability threshold trips because an orchestrator is down, which would otherwise deflate every node's score for a fault that is not theirs. A consequence MUST be documented: effective weights therefore differ by role, so a declared weight is a share of whatever applies rather than a fixed fraction.

The ENABLED properties' weights MUST sum to 1.0, checked at startup. At least one property MUST be enabled with a non-zero weight, and at least one of legacy v1 routing or liveness MUST be enabled: stress testing applies to mixnodes alone, so on its own it leaves every gateway with an empty applied set and no definable score. When a node's applied set IS empty, NO score may be computed for it.

Its previous annotation MUST then be carried over only when the gap is nym-api's own fault: this refresh's availability threshold dropped a component the node is eligible for by nym-api's current description of it, and a zero would slash it for an outage that is not its own. The carried annotation MUST keep its previous performance figures and MUST take the node's current rewarded-set role and chain capabilities. Every other node without a score MUST be dropped from the annotation map and is served 0, like a node never annotated: a node nym-api cannot describe is eligible for no component and scores config zero anyway, a node no longer bonded has nothing to be scored for, and a node whose gap is not nym-api's fault must not be served its last figure, because a node that has shut down would otherwise keep it. A carried annotation MUST have no age limit. A carried node is still described, so one that stops answering drops out with the next rebuild of the describe cache, whereas a cap would empty every annotation at once during an orchestrator outage longer than it.

Inertness MUST rest on the component's `enabled` flag defaulting to off, NOT on its weight defaulting to zero. A zero default weight was specified originally and is REJECTED: it is a second gate on the same property, and its only observable effect is a state in which the component reads as enabled while changing nothing, which is indistinguishable from a broken feature. The weight SHOULD therefore default to the value the component is expected to carry once switched on, making enabling a single change. The consequence MUST be documented where an operator will see it: enabling takes effect immediately at that weight, so the divergence surface is to be consulted BEFORE the flip rather than after.

While the weight is zero, nym-api MUST expose a DIVERGENCE surface comparing each node's aggregated liveness score against the v1 monitor's routing score, PER NODE, carrying the two scores, their difference, whether any liveness sample reached the node, and the node's declared role. It MUST list every liveness-eligible bonded node including those with no sample yet, so that coverage and comparison are readable together: the decision to weight liveness needs both how much of the fleet has been measured and how the measured part compares. It MUST omit nodes the orchestrator would never assign a liveness test, since those have no divergence to report rather than a divergence of zero. This surface is the evidence on which the eventual decisions to weight liveness, and separately to retire the v1 routing score, are to be based.

It MUST be served under the UNSTABLE route tree and is explicitly TEMPORARY: it exists to inform those two decisions and is to be deleted once they are made, so nothing durable may be built on it. With `use_performance_contract_data` on it MUST stay mounted and answer 503 naming nym-api's own figure as missing: contract mode computes no liveness or routing score of nym-api's own, and the contract-derived values its annotations then carry are not the comparison the surface exists for.

The only dimension beyond the per-node rows is the declared role, and only because nym-api already holds it. Bucketing on whether the node announces a wss entry was considered and REJECTED as unjustified complexity: it is inert for mixnodes, which announce no entry interface at all and will be most of the population, and it is second-order to the confound that actually dominates during rollout, namely whether a node yet carries the behaviour liveness depends on. The consequence is accepted rather than hidden: because this subsystem probes only the plain-ws ingress, a gateway with a broken TLS ingress will diverge for a reason this design created, and reading a gateway's divergence requires knowing that. The role field is what signals which of the two unrelated probe paths produced a row.

#### Scenario: Stress scores contribute to node performance when enabled
- **WHEN** stress-testing data is enabled and a mixnode has at least the minimum number of available results
- **THEN** its averaged stress score is folded into its detailed performance according to the configured weight

#### Scenario: Liveness scores are inert by default
- **WHEN** liveness results are stored and aggregated with the default configuration
- **THEN** they are queryable and appear on the divergence surface, but contribute nothing to any node's performance or reward, because the component's own enable flag is off

#### Scenario: A gateway is not docked for a test it cannot take
- **WHEN** stress testing is enabled and a gateway, which is never stress-tested, is scored
- **THEN** the properties that do apply are renormalised to carry the whole measurement, so the gateway is scored on what it has rather than losing the stress share

#### Scenario: Configuration gates every property
- **WHEN** a node with an imperfect config score scores well on a delivery property
- **THEN** that property's contribution is multiplied by the config score like every other, so a good delivery score cannot lift the node above its configuration ceiling

#### Scenario: A node with nothing applicable keeps its previous annotation
- **WHEN** no delivery property applies to a node because this refresh's availability threshold dropped a component the node is eligible for
- **THEN** no score is computed for it and its previous performance figures stand, under its current rewarded-set role and chain capabilities, rather than being replaced by a fabricated figure

#### Scenario: A node with nothing applicable through no fault of ours is dropped
- **WHEN** no delivery property applies to a node and no availability threshold dropped a component it is eligible for, as for a described node declaring no role while liveness is the only property enabled
- **THEN** it is left out of the annotation map and served 0, rather than keeping a figure from an earlier refresh

#### Scenario: An undescribed node is never carried over
- **WHEN** a node nym-api can no longer describe had an annotation, and this refresh's availability threshold dropped a component
- **THEN** it is left out of the annotation map, because it is eligible for no component, and is served 0

#### Scenario: Contract mode replaces nym-api's own annotations
- **WHEN** `use_performance_contract_data` is on, the annotation cache holds a legacy map loaded from disk, and a performance snapshot has settled
- **THEN** the next refresh replaces that map with annotations built from the contract's verified routing figure, reading no stress, liveness or routing data and computing no config score

#### Scenario: Divergence is reported per node
- **WHEN** a gateway scores zero on liveness while the v1 monitor scores it as routable
- **THEN** the surface reports that node's two scores, their difference, and its declared role, so the gap is attributable to that node and to which probe path measured it

#### Scenario: An unmeasured node is distinguishable from a broken one
- **WHEN** a liveness-eligible node has produced no sample in the window
- **THEN** it is still listed, with a zero score and its reachability flag false, so coverage is visible rather than being mistaken for a fleet of well-behaved nodes

#### Scenario: An ineligible node is absent rather than diverging
- **WHEN** a node declares neither a mixnode nor an entry role, so no liveness test is ever assigned to it
- **THEN** it does not appear on the surface at all

#### Scenario: The divergence surface is unavailable in contract mode
- **WHEN** the liveness divergence surface is requested with `use_performance_contract_data` on
- **THEN** it answers 503 naming nym-api's own figure as missing

#### Scenario: The consumer surface is bounded
- **WHEN** reasoning about the blast radius of a stress or liveness score
- **THEN** the readers are the performance aggregation query, the performance provider, and rewarding, each specified by its own capability

## ADDED Requirements

### Requirement: In contract mode nym-api publishes the verified routing figure as each node's performance

With the switch on, the node-status refresher SHALL build the annotation map from the latest settled performance snapshot (`performance-attestation-provider`): for every node in that snapshot's bond set with a committed entry, an annotation whose `performance_score` is the entry's committed score, or 0 when the entry has none, whose current role is the node's role in the current rewarded set, and which carries no chain capabilities. The topology routes publish a node's performance from `performance_score` alone, so in contract mode every one of them publishes exactly the value the contract's digest committed at that snapshot's height, which nym-api verified before signing it. A bonded node without a committed entry MUST have no annotation, and is served 0 like a node never annotated. No describe-based filter applies: whether a node is alive is the network monitor's measurement. The annotations MUST NOT carry the rewarding figure, which differs from the routing figure only after a partial measurement gap longer than the contract's 24-epoch lookback or in the minutes after a weights update, and which is on chain through `RewardingScore`.

The remaining performance fields are served only by the per-node annotation and performance routes, and MAY therefore carry values the digest does not commit. They SHALL come from what the producer read beside that snapshot, which holds medians for the entries of the newest epoch among its entries and of the epoch before it: the config, stress and liveness scores from the medians of the entry's bundle, each 0 when the kind has no median, with the stress and liveness reachability flags set exactly when their median is present; and the routing score recomputed as the weighted mean of the liveness and stress medians present, renormalised over their weights, under the weights in force at the snapshot's height, 0 when neither is present. An entry from an older epoch has no medians, and its sub-scores SHALL keep their defaults. The config score's breakdown fields have no source in the contract and SHALL keep their defaults. An entry without a score, a bundle the contract cannot score (no config median, or only zero-weighted kinds measured), SHALL be annotated with `performance_score` 0, its source epoch and its medians, so that the annotation shows it was measured and why it has no figure.

When no performance snapshot has settled, the refresh MUST leave the annotation map as it is and log, rather than publish a figure for no node.

#### Scenario: Published performance is the committed score
- **WHEN** node 7's committed entry in the latest settled snapshot is from epoch 10 with score 0.85
- **THEN** its annotation's `performance_score` is 0.85, and the skimmed routes publish it at 85%

#### Scenario: Sub-scores come from the entry's bundle medians
- **WHEN** node 7's entry is from the newest epoch among the snapshot's entries, its bundle there has liveness 0.9, stress 0.8 and config 0.95 medians, and the weights at the snapshot's height are liveness 0.8 and stress 0.2
- **THEN** its annotation carries liveness 0.9, stress 0.8, config 0.95 and routing 0.88, both reachability flags set

#### Scenario: A gateway's routing score is its liveness
- **WHEN** a gateway's bundle has liveness and config medians and no stress median
- **THEN** its stress score is 0 with the reachability flag unset, and its routing score equals its liveness median

#### Scenario: A node never measured has no annotation
- **WHEN** a node is in the snapshot's bond set but has no committed entry
- **THEN** it has no annotation and is served 0

#### Scenario: An entry from an older epoch keeps default sub-scores
- **WHEN** node 8's committed entry is from epoch 80 with score 0.7, and the newest epoch among the snapshot's entries is 100
- **THEN** its `performance_score` is 0.7 and its config, stress, liveness and routing scores keep their defaults

#### Scenario: An unscoreable entry is published at zero
- **WHEN** a node's committed entry has no score
- **THEN** its annotation's `performance_score` is 0 and it carries the entry's epoch and its bundle's medians

#### Scenario: An undescribed node is still published
- **WHEN** a node nym-api cannot describe has a committed entry with a score
- **THEN** it is annotated with that score

#### Scenario: No settled snapshot leaves the annotations as they are
- **WHEN** a contract-mode refresh runs while no performance snapshot has settled
- **THEN** the annotation map is left unchanged and the refresh logs it

### Requirement: Annotations carry their figure's source epoch and when it was computed

`DetailedNodePerformanceV2` SHALL carry two optional fields: `source_epoch`, the performance contract epoch the figure comes from, and `computed_at`, when the figure was taken. In contract mode both SHALL be set, `source_epoch` to the committed entry's epoch and `computed_at` to the block time of the height of the snapshot the figure was read from, so that it moves only when a newer snapshot settles and a stalled producer shows. In legacy mode `source_epoch` SHALL be absent and `computed_at` SHALL be the time of the refresh that computed the figure, a carried-over annotation keeping its own. Both MUST decode as absent from a response that lacks them, and MUST survive the annotation cache's persistence. The V1 annotation types SHALL be unchanged.

#### Scenario: A contract-mode annotation names its source epoch
- **WHEN** node 7 is annotated in contract mode from an entry of epoch 10 in the snapshot at height H
- **THEN** its `source_epoch` is 10 and its `computed_at` is the block time of H

#### Scenario: An unchanged snapshot keeps its time
- **WHEN** two contract-mode refreshes read the same latest settled snapshot
- **THEN** both give node 7 the same `computed_at`

#### Scenario: A carried legacy annotation keeps its computed time
- **WHEN** a legacy refresh carries node 7's annotation over because its coverage guard dropped a component node 7 is eligible for
- **THEN** the carried annotation keeps the `computed_at` of the refresh that computed it, and has no `source_epoch`

#### Scenario: V1 readers are unaffected
- **WHEN** a client reads `/v1/nym-nodes/annotation/{node_id}` in any output format
- **THEN** the response has the same shape as before this change

# node-geolocation-service Specification

## Purpose
TBD - created by archiving change verifiable-node-geolocation. Update Purpose after archive.
## Requirements
### Requirement: The service SHALL discover its subject set from the mixnet contract

Each cycle the service MUST query the mixnet contract for the set of bonded nym-nodes and treat those node ids as its subject set. Bonded nym-nodes are the only subject class the contract defines, so the service needs no other subject source; the discovery path MUST be structured so that adding a class later extends the subject set rather than reshaping the measurement, batching and submission paths.

A node that unbonds MUST drop out of the subject set on the next cycle, and the service MUST NOT submit further measurements for it.

#### Scenario: Newly bonded nodes enter the sweep
- **GIVEN** a node that bonds between two cycles
- **WHEN** the next cycle runs
- **THEN** it is included in the subject set and measured

#### Scenario: Unbonded nodes leave the sweep
- **GIVEN** a node present in the previous cycle that has since unbonded
- **WHEN** the next cycle runs
- **THEN** the service submits no measurement for it

### Requirement: The service SHALL discover addresses from node HTTP endpoints, and later from the directory contract

The service MUST determine each node's announced addresses by querying the node's own HTTP endpoint. A node MUST announce its addresses explicitly, so one that announces only a hostname is misconfigured and MUST be skipped rather than resolved: neither this service nor a client should be performing DNS lookups on a node's behalf.

The service MUST be structured so that the address source can be switched to the directory contract's node-published information without changing the measurement, batching or submission paths.

A node whose addresses cannot be discovered MUST be skipped for that cycle, MUST NOT cause the cycle to abort, and MUST be retried on the next cycle.

#### Scenario: An unreachable node does not abort the cycle
- **GIVEN** one node whose HTTP endpoint is unreachable
- **WHEN** the cycle runs
- **THEN** that node is skipped with a logged warning and every other node is still measured and submitted

#### Scenario: A hostname-only node is skipped
- **GIVEN** a node announcing a hostname and no addresses
- **WHEN** the cycle runs
- **THEN** the service submits no measurement for it and every other node is still measured

### Requirement: Discovered addresses SHALL NOT be persisted or exposed

Addresses obtained by the service MUST be held in memory only. They MUST NOT be persisted, MUST NOT be written to the contract, MUST NOT appear in durable logs, and MUST NOT be exposed on any service endpoint. The service does retain them in memory across cycles, because the address-change detection below compares against them, but that baseline MUST NOT outlive the process.

#### Scenario: A discovered address leaves no durable trace
- **GIVEN** a node that has been measured
- **WHEN** the service's persisted state and durable logs are inspected
- **THEN** the resolved address appears in neither

### Requirement: The service SHALL run a regular sweep and re-submit unchanged results

Freshness is a per-subject deadline rather than a whole-set cadence: a subject MUST be measured again once the `checked_at` stored for it is older than a configurable time-to-live, defaulting to approximately one month. The service MUST poll for subjects past that deadline on a much shorter interval, so one falling due is picked up by the next poll rather than waiting on a set-wide cycle.

Each poll is bounded by the per-sweep ceiling below, so a backlog larger than that ceiling MUST drain across successive polls rather than any subject being dropped: measuring a subject clears its deadline and removes it from the due set, so the following poll selects from what remains, and a subject whose measurement failed stays due and is retried. The ceiling therefore bounds a burst rather than throughput, and the polling interval and ceiling MUST together be able to clear the whole subject set well within the time-to-live.

It MUST submit a result even when the measured location is identical to the value already on-chain, so that `checked_at` advances and freshness stays verifiable.

#### Scenario: An unchanged measurement is still submitted
- **GIVEN** a node whose measured location equals the value already stored under this agent's key
- **WHEN** the regular sweep runs
- **THEN** the service still submits it, advancing `checked_at`

#### Scenario: A backlog larger than the per-sweep ceiling drains across polls
- **GIVEN** more subjects past their deadline than one sweep's ceiling permits
- **WHEN** successive polls run
- **THEN** each measures a further batch, and those it measured are no longer due, so the backlog drains rather than any subject being starved

### Requirement: The service SHALL trigger an explicit measurement when a subject's announced addresses change

The service MUST maintain a local baseline of the announced addresses it last measured for each subject and MUST trigger an out-of-band measurement when that set changes, without waiting for the next regular sweep.

Because addresses are not stored on-chain, this baseline is local to each agent and is lost on restart. On a cold start the service MUST treat its first sweep as the baseline and MUST NOT re-measure the whole subject set purely because it restarted; changes missed during downtime are caught by the next regular sweep.

#### Scenario: An address change triggers an immediate re-measurement
- **GIVEN** a node whose announced addresses differ from the service's baseline
- **WHEN** the service observes the change
- **THEN** it measures the node and submits the result without waiting for the regular interval

#### Scenario: A restart does not cause a full re-measurement
- **GIVEN** a service restarted with no local baseline
- **WHEN** its first sweep runs
- **THEN** it records the baseline and does not treat every subject as changed

### Requirement: The service SHALL submit results in batches sized to the contract limit

Results MUST be submitted in batches not exceeding the contract's `MAX_BATCH_SIZE`. The service MUST pre-validate entries against the contract's rules before submission, since batches are all-or-nothing.

Self-declaration relays MUST NOT share a transaction with measurements. A relay carries data the service did not produce and whose acceptance depends on contract state it does not control, so one bad artifact must never be able to fail a measurement sweep. Because relays arrive one at a time on their own endpoint rather than being gathered during a cycle, each is submitted in a transaction of its own and the separation is structural rather than a batching rule to observe.

The service MUST NOT rely on any particular ordering of entries within a batch.

#### Scenario: A rejected relay cannot lose measurements
- **GIVEN** a relay carrying an invalid signature, arriving while a measurement sweep is submitting
- **WHEN** it is rejected
- **THEN** no measurement batch is affected, because the relay was never part of one

### Requirement: The service SHALL relay node-signed self-declarations without modification

The service MUST expose an endpoint on which a node submits its own signed `NymNodeLocation` artifact, and MUST relay what it receives to the contract verbatim, preserving the node's signature and its `declared_at`. The service MUST NOT synthesise, re-sign or alter a self-declaration.

The node pushes rather than the service polling, because a declaration changes only when the operator changes it: polling every node on a schedule would spend a request per node per cycle to discover nothing almost every time, and would still relay a new declaration no sooner than the next poll. A pull path MAY be added later for nodes that cannot reach the service, and would reuse the same verification and relay.

Before relaying, the service MUST reject an artifact the contract would reject: one whose signature does not verify against the node's identity key, whose payload exceeds the configured size, whose `declared_at` is further ahead than the configured skew allows, or which is not newer than the declaration already stored. This is an optimisation and not the security control, which remains the contract's own verification; it exists so a bad artifact costs a rejected HTTP request rather than a failed transaction.

The artifact's location payload MUST already be the uniform `Location` shape the contract stores. The service MUST NOT widen, normalise or fill in a partial self-declaration, because doing so would change the signed bytes and because the resulting entry would then be attributable to the node while carrying data the node never asserted.

The service MUST forward the payload bytes exactly as received and MUST NOT parse and re-emit them, even when it parses a copy for logging or for change detection. JSON key ordering, whitespace and floating-point formatting vary between implementations, and the payload carries floating-point coordinates, so a re-serialised payload can differ from the signed original and fail verification on chain.

#### Scenario: Relayed bytes survive the round trip unchanged
- **GIVEN** a signed artifact fetched from a node
- **WHEN** the service relays it and the resulting entry is read back from the contract
- **THEN** the stored `content` is byte-for-byte identical to the bytes the node served, and the node's signature verifies against it

The service MUST tolerate the contract rejecting a relay as stale, since another agent may have relayed a newer artifact first. It MUST NOT treat that as a service failure, and MUST distinguish it from a relay that failed for any other reason, so that a node retrying against several agents is told its declaration is already on chain rather than that something went wrong.

#### Scenario: A stale relay rejection is benign
- **GIVEN** two agents relaying the same node's artifacts, where another agent has already submitted a newer one
- **WHEN** this agent's relay is rejected as stale
- **THEN** the service reports the conflict to the caller and stays healthy

### Requirement: The service SHALL expose an authenticated re-test endpoint with two authentication modes

The service MUST expose an HTTP endpoint that requests an out-of-band measurement of a given subject.

A request bearing the operator-held bearer token MUST be accepted without rate limiting.

A request signed by the target node's identity key MUST be accepted subject to the burst limit below, and MUST only ever trigger a measurement of that same node.

An unauthenticated request MUST be rejected.

#### Scenario: A node cannot request a re-test of a different node
- **GIVEN** a request signed by node 42's identity key naming node 43 as the subject
- **THEN** it is rejected

#### Scenario: The bearer token bypasses the burst limit
- **GIVEN** a node already locked out under the burst limit
- **WHEN** a bearer-token request names that node
- **THEN** the measurement is performed

### Requirement: Node-signed re-test requests SHALL be replay-protected

A node-signed re-test request MUST carry a timestamp inside the signed payload and MUST be rejected when that timestamp falls outside a short validity window. The service MUST additionally reject a request whose signature it has already seen within that window.

Without this, a captured request could be replayed to exhaust the target node's burst allowance, which is a cheap denial of service against a competitor.

#### Scenario: A replayed request is rejected
- **GIVEN** a valid node-signed request that has already been served
- **WHEN** the identical request is submitted again inside the validity window
- **THEN** it is rejected and the node's burst counter is unaffected

#### Scenario: An expired request is rejected
- **WHEN** a node-signed request whose timestamp is older than the validity window is submitted
- **THEN** it is rejected

### Requirement: Node-signed re-test requests SHALL be burst-limited per node and reset on a changed result

The service MUST count, per subject node, consecutive node-requested measurements that produced no change against the value currently stored under this agent's key. On reaching a configurable threshold, defaulting to three, the service MUST reject further node-signed requests for that node for a configurable cooldown, defaulting to one week.

The counter MUST reset as soon as a measurement produces a result differing from the stored value, so a node that genuinely relocates regains its allowance immediately.

Deciding whether a result changed MUST be done by reading the contract's current value for that subject, which keeps the decision correct regardless of the service's local state.

Measurements performed by the regular sweep or by a bearer-token request MUST NOT increment this counter, so a node cannot be locked out by the service's own activity.

The limit applies per agent. Each agent is an independent deployment holding its own counters, so a node's effective allowance across a fleet is the per-agent threshold multiplied by the number of agents. This is accepted.

#### Scenario: Three unchanged node-requested measurements trigger the cooldown
- **GIVEN** a node whose three consecutive node-requested measurements each returned the stored value
- **WHEN** it submits a fourth node-signed request
- **THEN** the request is rejected until the cooldown elapses

#### Scenario: A changed result restores the allowance
- **GIVEN** a node with two unchanged node-requested measurements recorded
- **WHEN** its next requested measurement returns a different location
- **THEN** the counter resets to zero

#### Scenario: The regular sweep does not consume the allowance
- **GIVEN** a node with no node-requested measurements recorded
- **WHEN** several regular sweeps measure it and return an unchanged result each time
- **THEN** its burst allowance is untouched

### Requirement: A geolocation lookup failure SHALL NOT be submitted as a result

When every address for a subject fails to geolocate, the service MUST NOT write an empty or placeholder location to the contract. It MUST leave the previous entry untouched, log the failure, and retry on the next cycle.

This is deliberately unlike the behaviour it replaces, where an unresolvable node yielded an empty location that removed a gateway from the dVPN directory.

#### Scenario: A failed lookup leaves the previous entry intact
- **GIVEN** a node with a stored measurement whose addresses now all fail to geolocate
- **WHEN** the cycle runs
- **THEN** no write is submitted for that node and its stored entry, including its `checked_at`, is unchanged

### Requirement: The service SHALL bound its consumption of the metered lookup provider

The service MUST apply a configurable ceiling on the subjects measured per sweep. Exhausting the provider's allowance MUST degrade to skipping measurements rather than aborting the cycle or crashing.

Because that degradation is indistinguishable from an ordinary lookup failure while affecting every subject at once, the service MUST log an error when the provider reports its quota exhausted. No further reporting is required: the service knows what it spends only as a count of requests it made, and the allowance itself lives on the provider account, so any figure it published would be a restatement of its own configuration rather than a measurement.

Repeated failures for one subject MUST NOT cause unbounded retries within a cycle.

#### Scenario: Allowance exhaustion degrades gracefully
- **GIVEN** a provider allowance that runs out partway through a sweep
- **WHEN** the cycle continues
- **THEN** the already-measured results are still submitted, the exhaustion is logged as an error, remaining subjects are skipped, and the service stays running


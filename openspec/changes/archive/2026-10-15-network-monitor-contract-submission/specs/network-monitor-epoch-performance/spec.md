## ADDED Requirements

### Requirement: A failed materialisation pass is retried within the epoch

When any step of a materialisation pass fails, whether materialising the aggregates, materialising the config scores, or the contract submission that follows them, the orchestrator SHALL start the next pass after the one-minute floor between looks at the chain rather than when the epoch in progress is due to end, and SHALL keep doing so until a pass completes. Each pass SHALL repeat every step. Only a pass in which every step succeeded SHALL wait for the epoch's end. A pass that skips its submission because the epoch's transition has already begun has not failed.

The performance contract accepts measurements only for the epoch in progress, so a pass that waited for the epoch's end after a failure would lose that epoch's submission, even though the failure was usually transient. Repeating every step is safe because each one is idempotent: an epoch whose aggregates are stored is not recomputed, a stored config score is left as it was, and submission resumes from the contract's cursor.

#### Scenario: A failed pass is retried within a minute
- **WHEN** a pass fails one minute into an epoch due to end in an hour
- **THEN** the next pass starts about a minute later, not when the epoch is due to end

#### Scenario: A completed pass waits for the epoch's end
- **WHEN** every step of a pass succeeds
- **THEN** the next pass starts when the epoch in progress is due to end

#### Scenario: A retry leaves stored figures unchanged
- **WHEN** a pass stored an epoch's aggregates and then failed to materialise its config scores
- **THEN** the retry stores the config scores and leaves the stored aggregates exactly as they were

#### Scenario: A stalled transition is not retried as a failure
- **WHEN** a pass materialises successfully and skips its submission because the epoch's transition has begun
- **THEN** the pass counts as complete and logs no error

## MODIFIED Requirements

### Requirement: Aggregates are served per kind on the orchestrator's HTTP API

The orchestrator SHALL expose materialised aggregates over HTTP, keyed by node and epoch, with liveness and stress reported separately alongside their evidence. The response shape MUST admit further entries without a breaking change.

Liveness and stress are kept separate because the weighting that combines them into a single performance figure is deliberately not the orchestrator's concern: the performance contract applies its own weights. Each entry is a named field of its own rather than a value in a uniform map, because entries do not all share the score-and-count shape: config score, served beside them, is a decomposition of its own.

The runs an aggregate was computed from are served per node and kind by the orchestrator's results endpoints, and nym-api holds the same per-run scores, so a value can be checked against its evidence without a surface of its own.

#### Scenario: Each entry is reported with its own value and evidence
- **WHEN** a consumer requests a node's aggregates for an epoch
- **THEN** it receives a separate liveness and stress entry, each carrying its value and sample count

#### Scenario: An entry with no aggregate is absent rather than zero
- **WHEN** a node has a stress aggregate and no liveness aggregate of either kind
- **THEN** the response omits the liveness entry rather than reporting it as 0.0

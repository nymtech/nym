## MODIFIED Requirements

### Requirement: Contract-dictated snapshot cadence

A producer SHALL read the snapshot interval from the directory contract and produce snapshots only at cadence heights (heights that are exact multiples of the interval), so that independent producers converge on identical snapshot heights without coordination. The producer SHALL NOT invent its own cadence. The interval SHALL be treated as configuration read from on-chain state, not as data committed into the directory digest or the signed snapshot. The producer SHALL read the interval afresh at every poll, together with the chain tip, rather than once at startup, so that an update to it moves every producer onto the new grid at its next poll instead of at its next restart, and SHALL place its heights on the grid through the cadence read shared with every other reader of the grid.

#### Scenario: Producer follows the contract interval

- **WHEN** the contract's snapshot interval is `Δ` and a producer observes the chain crossing a height `H` with `H mod Δ == 0`
- **THEN** the producer computes and signs a snapshot for height `H`, and does not produce snapshots at non-cadence heights

#### Scenario: Independent producers converge on the same heights

- **WHEN** two independently operated producers read the same interval `Δ` from the contract
- **THEN** both produce snapshots at the identical set of cadence heights, so a quorum can compare like-for-like

#### Scenario: An interval change is followed without a restart

- **WHEN** the directory contract's snapshot interval is updated on chain while a producer runs
- **THEN** the producer's next poll reads the new interval and produces at the new grid's heights

### Requirement: Deterministic production, retained window, and settle-lagged latest

A producer SHALL retain its snapshots at the most recent `N` cadence heights strictly below the chain tip it last polled (`N` a local configuration value), pruning by height rather than by count, so that producers retain the same heights whichever of them they failed to produce: every snapshot older than the oldest of those `N` heights is evicted. After an interval change, snapshots taken on the previous grid that are not older than that SHALL stay retained until the new grid's window passes them, so that the change never leaves "latest" without a snapshot to answer. The producer SHALL answer an explicit request for any retained cadence height immediately once that height is produced. The producer SHALL advertise as "latest" the greatest cadence height `H` such that the current chain tip is at least `H + settle_lag` blocks, where `settle_lag` is a small local configuration value in blocks. A freshly produced cadence height SHALL be explicitly queryable before it is advertised as latest.

In nym-api, `N` SHALL be `[directory.debug].retention_count` and `settle_lag` SHALL be `[directory.debug].settle_lag`, both shared by every contract the producer attests. The settle lag MUST NOT be taken from any other setting.

#### Scenario: Explicit height is served before it becomes latest

- **WHEN** a producer has just produced the snapshot for cadence height `H` and the tip has not yet reached `H + settle_lag`
- **THEN** an explicit query for height `H` returns that snapshot, while "latest" still returns the previous cadence height

#### Scenario: Latest advances after the settle lag

- **WHEN** the chain tip reaches `H + settle_lag`
- **THEN** "latest" advances to `H`

#### Scenario: Old snapshots fall out of the window

- **WHEN** more than `N` cadence snapshots have been produced
- **THEN** the producer retains only the most recent `N`, and an explicit query for an evicted height is reported as unavailable

#### Scenario: A missed height does not extend the window

- **WHEN** a producer failed to produce one of the `N` most recent cadence heights
- **THEN** it retains fewer than `N` snapshots rather than keeping an older height in its place

#### Scenario: The configured settle lag decides latest

- **WHEN** nym-api runs with `retention_count` 3 and `settle_lag` 10, and the tip is 5 blocks past the newest retained cadence height `H`
- **THEN** "latest" is still the cadence height before `H`, because the settle lag, not the retention count, decides it

#### Scenario: An interval change does not empty the window

- **WHEN** the interval changes and none of the new grid's `N` most recent heights is newer than the snapshots held
- **THEN** the held snapshots stay retained and "latest" keeps answering, each snapshot being evicted once it is older than the oldest of the new grid's `N` most recent heights

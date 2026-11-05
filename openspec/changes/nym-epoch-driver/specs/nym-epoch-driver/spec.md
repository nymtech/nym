## ADDED Requirements

### Requirement: The driver performs the epoch operations as the rewarding validator

`nym-epoch-driver` SHALL be the one process progressing the mixnet epoch. It MUST sign with the account the mixnet contract names as its rewarding validator and MUST refuse to start with any other. Its loop SHALL poll the current interval every 120 seconds, sleeping only the time left plus a short drift margin when less remains, and, once the epoch is over, run the epoch operations in the contract's order: begin the transition unless the contract reports it already begun by this account, bring the sealed epoch up to date in the window, pay the rewarded set, reconcile the pending events, select the next rewarded set and assign its roles. Each stage MUST read the contract's epoch state and skip what is already done, so that a restart or retry mid-epoch resumes at the stage that failed and never pays a node twice. A failed operation MUST be logged and retried after 30 seconds, the epoch staying marked as being advanced by this account. The rewarded set and the rewarding parameters for a payout, and the interval and epoch status, SHALL be read live from the chain at epoch time; a failed live read fails the operation and is retried, and nothing is substituted for it. The driver MUST NOT use a database.

#### Scenario: Another account is refused at startup
- **WHEN** the driver starts with a mnemonic whose account is not the contract's rewarding validator
- **THEN** startup fails naming both addresses, and no transaction is sent

#### Scenario: A failure during selection does not repay the epoch
- **WHEN** the role-assignment stage fails after the rewarded set was paid and events were reconciled
- **THEN** the retry 30 seconds later finds the contract in role assignment, sends no further `RewardNode` or reconciliation, and completes the assignment

#### Scenario: A restart mid-transition resumes
- **WHEN** the driver restarts while the contract reports the epoch as being advanced by its own account, in the rewarding state with nothing rewarded yet
- **THEN** it does not send `BeginEpochTransition` again, brings the window up to date, and continues from rewarding

#### Scenario: A failed live read substitutes nothing
- **WHEN** the rewarded-set read fails at epoch time
- **THEN** the operation fails and is retried, and no cached or previous rewarded set is paid

### Requirement: The driver holds a network view produced at the directory's snapshot cadence

The driver SHALL hold one network view: the node details of every bonded node (stake, identity key and bond information) and the verified directory, both read at a single height, with each node's roles derived from its directory record. The view SHALL be produced at the directory contract's snapshot cadence, by the epoch loop, which holds it: at each of its interval reads while the epoch is in progress, the loop reads the snapshot interval and the chain tip together, through the cadence read shared with every other reader of the grid, and produces the view at the newest cadence boundary that lies at least the settle lag below the tip and is newer than the one held. The directory MUST be verified against its digest at that height through the shared verified read, anchored on the block headers of the driver's own RPC node, which it always trusts, with the identity map taken from the node details already read, so that the bond set is read once per production. A production that fails for any reason MUST leave the previous view in place and be logged; selection reads the view as held. The time of the last successful production SHALL be what the driver reports as its network view's time.

#### Scenario: One read serves stake and identities
- **WHEN** a production runs at boundary height H
- **THEN** the node details are read once at H, the directory is verified at H against the identity keys of those details, and no second bond-set query is made

#### Scenario: A new boundary produces, an old one does not
- **WHEN** the tip advances past a cadence boundary by at least the settle lag
- **THEN** one production runs at that boundary, and no production runs again until the next boundary settles

#### Scenario: A failed production keeps the previous view
- **WHEN** the directory read at a boundary fails verification or the RPC call fails
- **THEN** the view produced at the previous boundary stays in place, the failure is logged, and the next boundary is tried in turn

#### Scenario: Selection reads the view as held
- **WHEN** an epoch ends while the newest settled boundary has not yet been produced
- **THEN** selection runs on the view held, without reading the directory at epoch time

### Requirement: Node roles come from the directory record's modes

A node's selectable roles SHALL be read from the `NodeInformation` label of its verified directory record: `modes.mixnode` makes it a mixnode candidate, `modes.entry` an entry-gateway candidate and `modes.exit` an exit-gateway candidate. The exit role MUST depend on `modes.exit` alone, not on the service providers the node publishes under its other labels. A bonded node with no `NodeInformation` record, or whose record carries no modes, MUST have no role and MUST NOT be a candidate for any. A node whose directory entries do not all verify against its bonded identity key, or whose `NodeInformation` record does not decode, MUST be treated as having no record. No other source of roles SHALL be consulted.

#### Scenario: Roles follow the published modes
- **WHEN** a bonded node's directory record declares mixnode and exit modes and no entry mode
- **THEN** it is a candidate for the mixnode and exit-gateway roles only

#### Scenario: Exit candidacy ignores the service providers
- **WHEN** a node declares the exit mode and publishes no network requester or IP packet router
- **THEN** it is an exit-gateway candidate

#### Scenario: A node without a record is skipped
- **WHEN** a bonded node has no `NodeInformation` record in the verified directory
- **THEN** it has no role and is not among the candidates for any role

### Requirement: The driver keeps a window of the performance contract's per-epoch medians

The driver SHALL keep a local window of the performance contract: for each epoch from `R - MAX_FALLBACK_LOOKBACK_EPOCHS` (24) to R, and for the current mixnet epoch X, every node's per-kind medians as `EpochPerformancePaged` returns them, together with the weights in force at X (`WeightsAt { epoch_id: X }`). R SHALL be the newest held epoch not after X in which any node has a bundle. The window MUST NOT keep the pages' scores, which are under each epoch's own weights rather than X's, and MUST hold an epoch read with no nodes as absent, so that it never counts as R.

At startup the window MUST read `LastSubmittedMeasurement` and the weights at X, and page the newest submission's epoch, even when X is lower because the interval read is behind, and each earlier epoch until one at or below X holds nodes, which is R, then every epoch down to `R - 24`. Submissions go only to the current epoch, so no bundle lies above the newest submission's epoch and no epoch above it is paged. That epoch is paged even above X because X can later reach it without a new submission to trigger a re-page. No epoch below `R - 24` is ever needed, however long an outage lasted, because the contract anchors its fallback at R. When none of the 25 epochs from where the walk starts holds nodes, the load MUST fail rather than walk on toward epoch 0. When the contract has never received a submission, the load MUST fail, rewarding and selection having no other figure. The window SHALL be held by the epoch loop alone, which MUST bring it up to date at each of its interval reads while the epoch is in progress, reading `LastSubmittedMeasurement` with the current epoch taken from that interval read; a failed update is logged and leaves the window as it was. When the current epoch has moved on, it MUST read the weights at the new X. When the last submission differs from the one last seen, it MUST re-page every epoch from the recorded submission's epoch to the new one's: only the current epoch takes submissions, but it can move on between two polls, so each epoch that was current at some point in between may have received one. It MUST then drop every epoch below `R - 24`. A poll MUST read everything it needs before changing the window, so that a failed poll leaves it as it was. The current epoch the window holds MUST never move backwards: a poll naming an older epoch keeps the one held. The last submission the window records MUST be the one read before the pages it triggered, so that a submission landing while they are read shows up as a change at the next poll rather than being masked.

The window is exact against the contract because only the current epoch is writable: every epoch below X is frozen once paged, and the epoch operations run one more poll once X is sealed. Its pages therefore need not be pinned to one height: a node written between two pages of X is picked up when the next poll sees the last submission change.

#### Scenario: A start after an outage loads only what the rule can reach
- **WHEN** the driver starts while the newest submission is for epoch 100, every epoch from 60 to 100 holds bundles, and the current epoch is 130
- **THEN** the window holds epochs 76 to 100 and the weights at 130, pages no epoch above 100, and holds epoch 130 as absent

#### Scenario: A newest submission whose epoch holds nothing
- **WHEN** the newest submission's epoch holds no bundle, because every node of its batch was skipped as not bonded, and the newest epoch holding bundles is two below it
- **THEN** the load walks down to that epoch as R and holds the 24 epochs below it as well

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

### Requirement: The driver resolves a node's contract figure with the contract's own resolution rule

The resolution of a node's source bundle for an epoch X that `performance-contract` specifies for `RewardingInputs` SHALL be one function in `nym-performance-contract-common`, called by the contract over its storage and by the driver over its window, so that the two cannot drift. The function MUST read its inputs through lookups it calls only when the rule needs them, in the contract's order (the bundle at X, then R, then the node's newest-bundle hint, then the walk), so that a hit at X still costs the contract one read. The hint MUST NOT change the result, and the driver, which holds no pointer, passes none.

The driver SHALL score the resolved source's medians with `Weights::score` under the window's weights at X, the score being absent when either is. A node's contract figure at X is its source epoch, the source's medians and that score, or nothing when the node has no source.

#### Scenario: The driver's figure equals the contract's RewardingInputs
- **WHEN** the window holds the bundles and weights the contract holds, for a node with a bundle at X, a node falling back to an earlier bundle, a node whose newest bundle lies below `R - 24`, and a node whose source cannot be scored
- **THEN** in each case the driver's figure at X has the source epoch, medians and score that `RewardingInputs { epoch_id: X, node_id }` returns

#### Scenario: The pointer hint never changes the source
- **WHEN** the rule runs over any layout of a node's bundles, once with the pointer the contract would hold for it and once without
- **THEN** both runs resolve the same source

### Requirement: Rewarding pays the contract's figure for the sealed epoch

The epoch operations SHALL compute the rewards for epoch X only after X's transition has begun, whether this run sent `BeginEpochTransition` or found it begun by this account after a restart, and MUST first bring X up to date in the window with one more poll, X being frozen from then on. That poll reads the contract's last submission and, only when it differs from the one the window recorded, re-pages every epoch from the recorded submission's epoch to X, normally X alone, so in the usual case it costs one query. Each node of the rewarded set MUST be paid its contract figure's score at X as its rewarding `Performance`, directly and unscaled. A node without a score at X, whether it has no source or its source cannot be scored, MUST be paid zero. A failed read of the contract MUST fail the epoch operation, which is retried, and MUST NOT pay any node zero in place of its figure.

#### Scenario: Rewards follow the sealed epoch
- **WHEN** the orchestrator's last chunk for X landed after the window's latest poll and X's transition then begins
- **THEN** the poll before the rewards finds the last submission changed and re-pages X, and every node of that chunk is paid from it

#### Scenario: An unchanged epoch is not re-read
- **WHEN** X's transition begins and the contract's last submission is the one the window recorded at its last poll
- **THEN** the poll before the rewards makes no query beyond that one read, and the rewards are computed from the window as it stands

#### Scenario: A node without a score is paid zero
- **WHEN** a rewarded node has no source at X, or a source the contract cannot score
- **THEN** it is paid a performance of zero

#### Scenario: A failed read pays no one
- **WHEN** the poll bringing X up to date fails after the transition has begun
- **THEN** no `RewardNode` is sent, the epoch operation fails and is retried, and the epoch stays marked as being advanced by this account

### Requirement: The active set is selected by the contract's figure, and by stake alone when nothing was submitted at the epoch

The rewarded set that follows epoch X SHALL be selected with each candidate weighted by `saturation x performance^20`, where performance is the candidate's contract figure's score at X from the window as brought up to date for rewarding, and saturation comes from the network view's node details. Candidates are the nodes bonded in the network view that have at least one role from their directory record. A node whose bond is already unbonding in the view MUST NOT be a candidate, since the epoch-end reconciliation, which runs before selection, removes it. A candidate without a score MUST be skipped. When nothing was submitted at X, that is R < X, every candidate with a score above 0 MUST be given a performance of 1, so that selection among them is by stake alone, and one warning MUST be logged. A candidate whose score is 0 MUST keep it: a node last measured completely dead stays out of the stake-weighted draw and remains available only to the sampler's uniform filler, as in normal operation.

A test kind the orchestrator omits is not that fault, because R keeps advancing on the other kinds. A node measured by that kind alone keeps its last figure for 24 epochs and is then skipped, its role being filled by the sampler's uniform filler.

#### Scenario: Candidates are weighted by their contract figure
- **WHEN** two candidates of equal saturation have contract scores of 0.9 and 0.5 at X
- **THEN** they are weighted `saturation x 0.9^20` and `saturation x 0.5^20`

#### Scenario: Selection falls back to stake when nothing was submitted
- **WHEN** no bundle exists at X and the candidates' scores come from bundles at R, one of them 0
- **THEN** every candidate with a score above 0 is weighted by its saturation alone, the candidate scoring 0 keeps weight 0, and a warning is logged

#### Scenario: A candidate without a score is skipped
- **WHEN** a candidate has no score at X, with or without a bundle at X for other nodes
- **THEN** it is not among the weighted candidates

#### Scenario: Candidates come from the network view
- **WHEN** a node bonded after the network view's height has a score at X
- **THEN** it is not a candidate until a later view includes it

#### Scenario: An unbonding node is not a candidate
- **WHEN** a node's bond is marked unbonding in the network view, though it has a role and a score at X
- **THEN** it is not among the candidates for any role

### Requirement: Startup checks the account, the window, the network view and the directory address

Before its loop starts, the driver MUST, in order: confirm that its signing account is the mixnet contract's rewarding validator; confirm the directory and performance contract addresses are present in the network details; load the window, failing when it cannot be loaded or when the contract has never received a submission, because rewarding and selection have no other figure; and produce the network view at the newest settled cadence boundary, failing when that production fails. Startup MUST fail on any of these rather than start a loop that would pay nothing or select nobody. The driver has no caches to wait for.

#### Scenario: A contract that never received a submission stops the start
- **WHEN** the contract's last submission carries no data
- **THEN** startup fails before the loop starts

#### Scenario: A missing contract address fails startup
- **WHEN** the network details carry no directory contract address or no performance contract address
- **THEN** startup fails naming the missing contract

#### Scenario: A failed first production fails startup
- **WHEN** the network view cannot be produced at startup because the directory read fails verification
- **THEN** startup fails, and no epoch operation runs on an empty view

### Requirement: The driver is configured by a file with environment and flag overrides

The driver SHALL be configured by a file that `init` writes under `~/.nym/nym-epoch-driver/`, at one location with no instance id, and `run` loads, with values overridden by environment variables and then by flags, in that precedence. The file MUST carry the rewarding account's mnemonic, the nyxd URL and the metrics bearer token, none of them optional: `init` MUST refuse to write a file without the mnemonic or the token; the contract addresses come from the network environment. The network view's settle lag SHALL be a tuning knob in the file's debug section, overridable by environment and hidden from the command line's help, defaulting to the value nym-api's directory settings use. The network view's cadence comes from the chain and SHALL NOT be configurable.

#### Scenario: An override wins over the file
- **WHEN** the file names one nyxd URL and the environment names another
- **THEN** the driver connects to the environment's URL, and a flag naming a third wins over both

#### Scenario: The cadence is not a knob
- **WHEN** the directory contract's snapshot interval changes
- **THEN** the driver follows it after a restart, with no configuration change

### Requirement: The driver serves Prometheus metrics behind a bearer token

The driver SHALL serve Prometheus metrics on a listener bound, by default, to all interfaces (`0.0.0.0`), behind a bearer token, rejecting requests without it. The token MUST be configured: when, after the overrides, it is missing or empty, the driver MUST refuse to start. It MUST publish three times, each as a Unix timestamp so that an alert compares it with the current time: the end of the epoch in progress as its loop last read it from the chain, so that an epoch which has ended and not advanced shows as that end falling ever further behind; the time of its loop's last interval read, so that a driver hung while still answering requests shows as that time falling behind; and the time of its last successful network-view production, so that selection running on an old view, which nothing on chain shows, is visible. It SHALL serve no other route.

#### Scenario: Metrics need the token
- **WHEN** the metrics route is requested without the configured bearer token
- **THEN** the request is rejected as unauthorised and no metrics are served

#### Scenario: No token, no start
- **WHEN** the driver starts with no metrics bearer token configured, or an empty one
- **THEN** startup fails before any chain read, and no listener is bound

#### Scenario: A stalled epoch shows in the metrics
- **WHEN** the epoch in progress has ended and every attempt at its operations fails
- **THEN** the published epoch end stays at that epoch's end, falling further behind the current time, while the loop's time keeps advancing with each retry

#### Scenario: A stale network view shows in the metrics
- **WHEN** the network view's productions fail across several cadence boundaries
- **THEN** the published network-view time stays at the last successful production

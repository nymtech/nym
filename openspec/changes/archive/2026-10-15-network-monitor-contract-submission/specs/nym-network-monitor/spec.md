## RENAMED Requirements

- FROM: `### Requirement: The orchestrator runs four long-lived tasks and rehydrates its agent set from the contract at startup`
- TO: `### Requirement: The orchestrator runs six long-lived tasks and rehydrates its agent set from the contract at startup`

## MODIFIED Requirements

### Requirement: The orchestrator gates startup on chain balance, contract authorisation, identity-key reconciliation, and nym-api recognition

Each network-monitor orchestrator SHALL be an independent daemon holding an ed25519 identity keypair and a Nyx (bip39) account. On startup it MUST complete the following gates in order before serving, aborting startup on any failure: initialise its SQLite database; build a direct-signing nyxd client from its mnemonic; verify its signing account holds at least `1_000_000 unym` (to cover its on-chain transaction fees); verify its bech32 address is present in the network-monitors contract's authorised-orchestrator set, retrying up to `chain_authorisation_check_max_attempts` (default 10) spaced by `chain_authorisation_check_retry_delay` (default 1 minute); only when its configuration enables performance submission, verify its account is an authorised network monitor in the performance contract, with the same retry budget, as `network-monitor-contract-submission` specifies; reconcile its announced identity key, submitting an `UpdateOrchestratorIdentityKey` transaction and sleeping 30 seconds when the on-chain key differs from (or is missing versus) its local key; and verify nym-api recognises it as authorised via the `known-monitors` self-check endpoint.

A fleet MAY run more than one orchestrator; each orchestrator manages only the agents it authorised (`authorised_by == self`).

#### Scenario: An unauthorised orchestrator does not start serving
- **WHEN** an orchestrator's bech32 address is not in the contract's authorised set after the configured retries
- **THEN** startup aborts and no HTTP server or background task begins

#### Scenario: A stale on-chain identity key is reconciled before serving
- **WHEN** the identity key announced on-chain differs from the orchestrator's local key
- **THEN** the orchestrator submits `UpdateOrchestratorIdentityKey`, waits for it to take effect, and only then continues startup

#### Scenario: Insufficient balance aborts startup
- **WHEN** the signing account holds fewer than `1_000_000 unym`
- **THEN** startup aborts before any agent can be authorised on-chain

### Requirement: The orchestrator runs six long-lived tasks and rehydrates its agent set from the contract at startup

On a successful startup the orchestrator SHALL rebuild its in-memory agent registry from the contract (all agents whose `authorised_by` equals its own address) and then spawn exactly six long-lived tasks under a shutdown manager: an HTTP server (serves continuously); a node refresher on `node_refresh_rate` (default 2 hours, first tick immediate); a result submitter on `result_submission_interval` (default 15 minutes, first tick offset by one interval); a stale-data eviction task on a derived cadence of roughly 2 minutes 30 seconds at default settings; a chain-capability refresher, waking every 5 minutes (first tick immediate) to re-query the nodes whose cached on-chain standing is missing or due; and a materialiser, whose first pass runs immediately and each later pass when the epoch in progress is due to end, or after a one-minute floor when the previous pass failed, the interval could not be read, or the epoch is overdue. Submission to the performance contract, when on, runs as the last step of the materialiser's pass rather than as a task of its own. The eviction task MUST additionally run one blocking sweep before serving, to release in-progress rows left behind by a prior crash.

#### Scenario: The full task set is spawned after gating
- **WHEN** an authorised orchestrator finishes its startup gates
- **THEN** it rehydrates its agent set from the contract and spawns the HTTP server, node refresher, result submitter, stale-data eviction, chain-capability refresher, and materialiser tasks

#### Scenario: A prior crash's in-flight rows are released on boot
- **WHEN** the orchestrator restarts with `testrun_in_progress` rows left over from a previous run
- **THEN** one blocking eviction sweep runs before serving to clear timed-out in-progress dispatches

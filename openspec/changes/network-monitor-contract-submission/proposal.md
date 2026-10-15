## Why

The orchestrator materialises every epoch's liveness, stress and config-score figures, and the performance contract can now hold them, commit them to a verifiable digest and fall back to last-known values during an outage (`performance-contract-verifiable-routing-view`, `performance-retrieval-client`). Nothing sends the figures to the contract yet. This change submits them every epoch, guarded so that a fault on the measuring side becomes missing data rather than a network of zeros. It also fixes the materialiser sleeping until the epoch's end after a failed step: that is harmless for the HTTP endpoints, which the next pass backfills, but fatal for submission, because the contract accepts only the epoch in progress.

## What Changes

- Submission is the final step of the materialiser's per-epoch pass: the epoch in progress only, taken from the same interval reading the figures were materialised against, only after both materialisation steps succeeded, and only while the epoch's transition has not begun. Backfilled epochs are never submitted.
- A node is submitted with measured values only (config score, combined liveness, stress), never zero-filled, and only with a config score and at least one of liveness or stress.
- A coverage guard per test kind (mixnode liveness, gateway liveness, stress): a test kind whose share of eligible described nodes with a reachable aggregate falls below a threshold (default 0.5) is left out for every node before liveness is combined, so the contract renormalises over what is left or, with nothing submitted, falls back to last-known values.
- Nodes go out in ascending node order as chunked `BatchSubmit` transactions (default 200 nodes each), resuming after a restart or a failed chunk from the contract's own submission cursor.
- A materialisation pass with any failed step, submission included, is retried after the one-minute floor instead of at the epoch's end. A pass that skips submission because the epoch is in transition has not failed.
- Submission is on exactly when the configuration enables it (`--performance-submission-enabled`, off by default); the network details are assumed to carry the performance contract's address. Startup then verifies, right after the network-monitors contract check and with the same retry budget, that the account is an authorised network monitor in the performance contract, and aborts otherwise.
- Signals: the newest epoch whose submission completed, so an incomplete epoch alerts when it stops advancing, and the test kinds the guard left out. A failed describe of a node is logged at `info` instead of `debug`, so every node missing from a submission can be matched to its reason.
- New configuration: the submission switch, the submission batch size and the coverage threshold.
- The epoch-performance spec loses its stale sentence naming config score as the next candidate to move out of nym-api, which has since happened.
- The `nym-network-monitor` spec's startup gates gain the performance contract check, and its task-list requirement, which still names four tasks where six run, is corrected to list the chain-capability refresher and the materialiser.

## Capabilities

### New Capabilities

- `network-monitor-contract-submission`: the orchestrator's submission of per-epoch measurements to the performance contract: when it is enabled and the startup authorisation check, which epoch is submitted and when, what a node's submission carries, the coverage guard, ordering, chunking and cursor resume, and the signals that make an incomplete epoch and a guard trip visible.

### Modified Capabilities

- `network-monitor-epoch-performance`: adds the retry of a failed materialisation pass within the epoch, and rewords the HTTP API requirement's rationale to drop the stale "config score is the next candidate" sentence (its scenarios are unchanged).
- `nym-network-monitor`: the startup-gates requirement gains the conditional performance contract authorisation check right after the network-monitors contract check; the task-list requirement is renamed from four to six long-lived tasks and lists the chain-capability refresher and the materialiser, with submission running inside the materialiser's pass.

## Impact

**Code**, all in `nym-network-monitor-v3/nym-network-monitor-orchestrator`:
- `aggregation/materialiser/mod.rs`: the pass and its retry, the epoch-status check, and the optional submission step;
- a new `submission` module: building an epoch's submission from the materialised rows, the coverage guard, the cursor resume and the chunk loop, and a shared-client handle implementing the validator client's performance traits;
- `aggregation/mod.rs` and `http/state.rs`: the per-node union of config-score and aggregate rows moves into `aggregation`, so the HTTP API and the submission read one fold;
- `storage`: a read of the described nodes' roles;
- `orchestrator/mod.rs` and `orchestrator/config.rs`: the startup authorisation check and the new settings; `cli/run_orchestrator.rs` and `cli/env.rs`: their flags;
- `orchestrator/node_refresher.rs`: the describe-failure log level; `orchestrator/prometheus.rs`: the submission series;
- `Cargo.toml`: `nym-contracts-common` with `naive_float`, for the percent conversion.

No contract, message, wire type, HTTP response or database schema changes. The submission cursor lives on chain, so the orchestrator keeps no new state.

**Deployment**: nothing changes until submission is enabled. Turning it on needs the contract deployed with the mixnet migration, the orchestrator's account authorised in it and funded (roughly 0.3 NYM per epoch at ~1000 nodes), the address in the orchestrator's network details, and `--performance-submission-enabled true`.

**Consumers**: none in this change. nym-api reads the submitted data in `nym-api-performance-contract-consumer`.

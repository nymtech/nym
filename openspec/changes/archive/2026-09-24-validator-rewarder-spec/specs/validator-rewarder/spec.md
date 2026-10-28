## ADDED Requirements

### Requirement: The rewarder runs only with at least one rewarding module enabled

`nym-validator-rewarder` SHALL host exactly two rewarding modules, block signing (`config.block_signing`) and ticketbook issuance (`config.ticketbook_issuance`), each independently switchable by its own `enabled` flag and independently restrictable by its own `monitor_only` flag. Construction MUST fail with `RewardingModulesDisabled` when both modules are disabled. A disabled module MUST NOT be constructed, MUST NOT measure anything, and its periodic handler MUST return immediately; the other module MUST continue to operate normally.

#### Scenario: Both modules disabled refuses to start
- **WHEN** the rewarder is started with `block_signing.enabled = false` and `ticketbook_issuance.enabled = false`
- **THEN** startup fails with `RewardingModulesDisabled` and no measurement, transaction or database write occurs

#### Scenario: A single enabled module runs alone
- **WHEN** only `block_signing.enabled` is `true`
- **THEN** the block-signing module is constructed and its epoch ticker runs
- **AND** no ticketbook issuer is ever queried and no issuance row is written

#### Scenario: A disabled module's handler is inert
- **WHEN** the daily issuance ticker fires while `ticketbook_issuance.enabled` is `false`
- **THEN** the handler returns without querying issuers, sending a transaction, or writing an issuance epoch row

### Requirement: An enabled module requires a non-empty whitelist

Each module SHALL carry its own whitelist in its own address format: `block_signing.whitelist` holds `nvalcons1…` validator consensus addresses, and `ticketbook_issuance.whitelist` holds `n1…` operator accounts. Construction MUST fail with `EmptyBlockSigningWhitelist` or `EmptyTicketbookIssuanceWhitelist` when the corresponding module is enabled and its whitelist is empty. The whitelists MUST NOT be merged, shared, or defaulted from one another.

#### Scenario: Enabled block signing with an empty whitelist refuses to start
- **WHEN** `block_signing.enabled` is `true` and `block_signing.whitelist` is empty
- **THEN** startup fails with `EmptyBlockSigningWhitelist`

#### Scenario: Enabled issuance with an empty whitelist refuses to start
- **WHEN** `ticketbook_issuance.enabled` is `true` and `ticketbook_issuance.whitelist` is empty
- **THEN** startup fails with `EmptyTicketbookIssuanceWhitelist`

#### Scenario: A disabled module's empty whitelist is not an error
- **WHEN** `ticketbook_issuance.enabled` is `false` and its whitelist is empty, while block signing is enabled with a populated whitelist
- **THEN** startup succeeds

### Requirement: An ed25519 identity keypair is required unconditionally at startup

The rewarder SHALL load an ed25519 identity keypair from `storage_paths` before either module is constructed, and MUST fail startup when it cannot be loaded, regardless of which modules are enabled. The load failure MUST be logged with the remediation hint naming the `regenerate-identity` subcommand. The keypair is the identity with which the rewarder signs its ticketbook issuance challenges, so it MUST be stable across restarts: rotating it invalidates nothing already recorded, but any evidence blob referencing the retired public key can no longer be tied to the running instance.

#### Scenario: Missing identity keys refuse to start even for block signing only
- **WHEN** the configured key files are absent and only `block_signing.enabled` is `true`
- **THEN** startup fails with the ed25519 load error and logs the `regenerate-identity` hint

#### Scenario: Loaded keypair is used to sign issuance challenges
- **WHEN** the ticketbook issuance module issues a deposit challenge
- **THEN** the request body is signed with the loaded private key and the corresponding public key is embedded in any evidence produced during that audit

### Requirement: Configuration is validated before any rewarding, and rewarding ratios must sum to exactly 1.0

Configuration SHALL be validated on load and again by the subcommand wrapper before a `Rewarder` is constructed. Validation MUST reject a `rewarding.ratios` triple (`block_signing`, `ticketbook_issuance`, `ticketbook_verification`) whose sum is not exactly `1.0` with `InvalidRewardingRatios`, using exact floating-point comparison. Validation MUST also validate the scraper pruning configuration against the configured block-signing epoch duration. A configuration file that fails to parse MUST be passed through `try_upgrade_config`, which is currently a no-op returning success, before the load is retried and reported.

#### Scenario: Ratios that do not sum to one are rejected
- **WHEN** the configuration sets `block_signing = 0.7` and `ticketbook_issuance = 0.33`
- **THEN** validation fails with `InvalidRewardingRatios` and the process does not start

#### Scenario: Default ratios are accepted
- **WHEN** the configuration carries the defaults `block_signing = 0.67`, `ticketbook_issuance = 0.33`, `ticketbook_verification = 0.0`
- **THEN** validation succeeds

#### Scenario: Unparseable configuration reports a load failure
- **WHEN** the configuration file cannot be read or parsed
- **THEN** `try_upgrade_config` runs, the load is retried, and the failure is reported with the hint that `init` may not have been run

### Requirement: Scraper pruning must retain enough blocks to cover a block-signing epoch

Because block signing is measured from the embedded scraper's stored blocks and pre-commits, pruning configuration SHALL be constrained at startup. A `nothing` strategy MUST be accepted unconditionally. An `everything` strategy MUST be rejected with `EverythingPruningStrategy`. A `custom` strategy MUST be rejected with `TooSmallKeepRecent` unless `keep_recent >= ceil(epoch_duration_secs / TYPICAL_BLOCK_TIME * 1.5)`, where `TYPICAL_BLOCK_TIME` is 5 seconds. When the `pruning` field is absent from the configuration file it MUST default to `nothing`, preserving the behaviour of versions that had no pruning support. The scraper MUST be configured with `store_precommits` (default `true`), without which no signing measurement is possible.

#### Scenario: Everything-pruning is rejected
- **WHEN** `nyxd_scraper.pruning.strategy = 'everything'`
- **THEN** validation fails with `EverythingPruningStrategy`

#### Scenario: Custom pruning below the epoch requirement is rejected
- **WHEN** the epoch duration is 1 hour and `pruning.strategy = 'custom'` with `keep_recent = 500`
- **THEN** validation fails with `TooSmallKeepRecent` reporting a minimum of 1080 blocks

#### Scenario: Absent pruning configuration defaults to archiving
- **WHEN** the configuration file contains no `pruning` section
- **THEN** the strategy is `nothing` and validation succeeds

### Requirement: A rewarder that will send rewards requires seven days of budget on hand

When `will_attempt_to_send_rewards()` is true - that is, when at least one enabled module is not in `monitor_only` mode - startup SHALL query the rewarding account's balance in the budget denomination and MUST fail with `InsufficientRewarderBalance` unless the balance is at least seven times `rewarding.daily_budget`. The error MUST report the daily budget, the observed balance and the required minimum. When every enabled module is in `monitor_only` mode the balance MUST NOT be checked.

#### Scenario: Under-funded account refuses to start
- **WHEN** the daily budget is 24000 NYM, the account holds 100000 NYM, and block signing is enabled without `monitor_only`
- **THEN** startup fails with `InsufficientRewarderBalance` reporting a 168000 NYM minimum

#### Scenario: Monitor-only deployment skips the balance check
- **WHEN** every enabled module has `monitor_only = true`
- **THEN** no balance query is made and startup proceeds regardless of the account's funds

### Requirement: Category budgets are derived from the daily budget, and the issuance budget is split into equal per-operator slices

The rewarder SHALL derive every budget from `rewarding.daily_budget` and `rewarding.ratios`, in the budget's own denomination (default `24000000000 unym` per day):

- The block-signing budget for one epoch MUST be `floor(daily_budget.amount * ratios.block_signing * epoch_duration_secs / 86400)`.
- The ticketbook issuance budget for one day MUST be `floor(daily_budget.amount * ratios.ticketbook_issuance)`.
- The per-operator issuance budget MUST be `floor(issuance_daily_budget / whitelist_size)` computed with `Decimal::from_ratio(1, whitelist_size)` and `mul_floor`, and MUST be zero when the whitelist is empty.

The per-operator budget MUST be logged at computation time together with the total daily budget, the issuance budget and the whitelist size. Both budgets MAY be under-spent and the remainder MUST NOT be redistributed.

#### Scenario: Hourly epoch budget is prorated
- **WHEN** the daily budget is 24000 NYM, `ratios.block_signing` is 0.67 and the epoch duration is 1 hour
- **THEN** the epoch budget is 670 NYM

#### Scenario: Issuance budget is divided equally by whitelist size
- **WHEN** the daily budget is 24000 NYM, `ratios.ticketbook_issuance` is 0.33 and the issuance whitelist holds 4 accounts
- **THEN** the issuance daily budget is 7920 NYM and the per-operator budget is 1980 NYM

#### Scenario: Unclaimed budget is not redistributed
- **WHEN** a period's computed rewards total less than the period's budget
- **THEN** only the computed total is sent and the remainder stays in the rewarding account

### Requirement: Monitor-only mode performs the full measurement and records it without sending a transaction

A module with `monitor_only = true` SHALL execute its entire measurement path, compute every reward amount, and persist the period and its per-participant rows exactly as it would when paying, but MUST NOT send a transaction. In that case the recorded transaction hash MUST be absent, the recorded spend MUST be zero, and the period MUST be flagged `monitor_only`. `monitor_only` is per module: one module MAY pay while the other only monitors.

#### Scenario: Monitored epoch is measured and recorded but unpaid
- **WHEN** a block-signing epoch ends with `block_signing.monitor_only = true`
- **THEN** signing ratios and reward amounts are computed and written per validator
- **AND** no transaction is sent, the recorded spend is zero, and the details row is flagged `monitor_only`

#### Scenario: Mixed modes are honoured independently
- **WHEN** `block_signing.monitor_only = true` and `ticketbook_issuance.monitor_only = false`
- **THEN** block-signing rewards are only recorded while ticketbook issuance rewards are sent on chain

### Requirement: The main loop services shutdown, scraper loss, the epoch ticker and the daily ticker, in that priority

The rewarder SHALL run a single biased `select!` loop over four events, evaluated in this order: the process shutdown signal, cancellation of the nyxd scraper, the block-signing epoch ticker, and the daily ticketbook issuance ticker. Cancellation of the scraper MUST break the loop, because a stalled scraper would otherwise yield measurements from an incomplete chain view. On exit the scraper MUST be stopped, cancellation MUST be propagated if it did not originate from the shutdown manager, and the process MUST wait for tasks to finish. The epoch ticker MUST be armed to fire at the current epoch's end and then at `epoch_duration` intervals; the daily ticker MUST be armed to fire at the next midnight UTC plus two hours of leeway and then every 24 hours.

#### Scenario: Shutdown takes priority over a due tick
- **WHEN** the shutdown signal arrives in the same poll as a ready epoch tick
- **THEN** the loop breaks without processing the epoch

#### Scenario: Losing the scraper stops the rewarder
- **WHEN** the nyxd scraper's cancellation token fires
- **THEN** a warning is logged, the loop breaks, and the process shuts down rather than rewarding from a stalled database

#### Scenario: Daily ticker fires after midnight with leeway
- **WHEN** the process starts at 09:00 UTC
- **THEN** the issuance ticker is armed for 02:00 UTC the next day and repeats every 24 hours

### Requirement: Block-signing epochs are contiguous and resumed from storage, and finished epochs are replayed on startup

Block signing SHALL be organised into contiguous epochs identified by a monotonically increasing `id`, where each epoch's `start_time` is the previous epoch's `end_time` and its `end_time` is `start_time + epoch_duration`. On startup the current epoch MUST be the successor of the newest epoch in storage, or, when storage holds none, epoch `0` starting at `now + 1 hour` truncated to the hour. Before entering the main loop the rewarder MUST replay every already-finished epoch that has no stored header, in order, rewarding each, until the current epoch is still in progress. The epoch marker is the header row, which MUST be written before any reward transaction is sent (see the persistence requirement), so an epoch whose header exists is resumed past and never replayed. A header written without a settled details row MUST be reported at startup as possibly unsettled, not replayed.

#### Scenario: Fresh deployment starts at the next hour boundary
- **WHEN** the rewarder starts at 10:17 UTC with an empty database and a 1 hour epoch duration
- **THEN** epoch 0 runs from 11:00 to 12:00 UTC and the first rewarding happens at its end

#### Scenario: Downtime is replayed epoch by epoch
- **WHEN** the rewarder restarts after being down for three epochs
- **THEN** each missed epoch is measured and rewarded in order before the main loop starts, and each is recorded separately

#### Scenario: Epoch marker advances past a failed epoch
- **WHEN** an epoch's rewarding fails
- **THEN** the failure is recorded for that epoch id and the next epoch becomes current, with no retry of the failed one

### Requirement: A startup replay that finds no blocks for an epoch aborts with the documented recovery runbook

Before replaying a finished epoch the rewarder SHALL verify that the scraper holds at least one block after the epoch's start and at least one block before its end, and MUST abort startup with `NoBlocksProcessedInEpoch` when either is missing. On that failure the process MUST log the five-step operator runbook: determine the epoch's approximate first height, run `process-until --start-height=$STARTING_BLOCK`, temporarily set `pruning.strategy = nothing`, restart the rewarder until the missing rewards are sent, then re-enable pruning and restart again. Automatic backfill is explicitly not implemented.

#### Scenario: Gap in scraped history aborts startup with instructions
- **WHEN** startup replay reaches an epoch for which the scraper database holds no blocks
- **THEN** startup fails with `NoBlocksProcessedInEpoch` naming the epoch
- **AND** the five-step recovery runbook is logged

### Requirement: The ticketbook issuance day is resumed from the last processed expiration date and processed at most once

Ticketbook issuance SHALL be keyed by ticketbook expiration date, and each daily run MUST process the cohort whose expiration date is the day before the current ecash day. The last processed expiration date MUST be loaded from storage on startup, defaulting to yesterday when storage holds none. The marker is the header row, which MUST be written before any reward transaction for the day is sent (see the persistence requirement), so a day whose header exists is resumed past and never re-audited, and a header written without a settled details row MUST be reported at startup as possibly unsettled, not replayed. A run whose target date is not newer than the marker MUST log that the date was already processed and return without querying any issuer, which makes a crash-restart within the same day idempotent.

#### Scenario: Cohort is the previous day's expiration date
- **WHEN** the daily handler runs at 02:00 UTC on 2026-09-21
- **THEN** it audits the ticketbooks whose expiration date is 2026-09-20

#### Scenario: Already-processed day is skipped
- **WHEN** the handler runs and the stored marker is equal to or newer than the target date
- **THEN** it logs that the date was already processed and performs no queries and no payment

#### Scenario: Fresh database never audits the day before startup
- **WHEN** the rewarder starts with an empty database on 2026-09-21, so the marker defaults to 2026-09-20
- **THEN** the 2026-09-20 cohort is never audited
- **AND** the run at 02:00 UTC on 2026-09-22 is the first to process a cohort, namely 2026-09-21

### Requirement: Each module settles a period with a single multi-recipient bank transaction

Settlement SHALL build a list of `(recipient, [coin])` pairs, omitting every participant whose computed amount is zero, and send it as one `send_multiple` bank transaction with a human-readable memo, recording the resulting hash against the period. An empty list MUST NOT be sent and MUST instead produce `NoValidatorsToReward` or `NoSignersToReward` for that period. Before sending, the rewarder MUST reject the whole transaction with `EmptyRewardingCoin`, logged as an error, if any coin list is empty or any amount is zero. The total spend recorded for a period MUST be the sum of the sent amounts, and MUST be zero whenever no transaction was sent.

#### Scenario: One transaction pays every eligible participant
- **WHEN** an epoch computes non-zero rewards for five validators
- **THEN** a single `send_multiple` transaction carrying five recipients is sent and its hash is recorded for the epoch

#### Scenario: Nothing to pay is an error for the period, not a transaction
- **WHEN** every computed amount for a period is zero
- **THEN** no transaction is sent and the period records `NoValidatorsToReward` or `NoSignersToReward`

#### Scenario: Memos name the period being paid
- **WHEN** a block-signing epoch is settled
- **THEN** the memo reads `block signing rewards for epoch <id>`
- **AND** when an issuance cohort is settled the memo reads `ticketbook issuance rewards for expiration date <date>`, naming the cohort being paid

### Requirement: Every period is persisted as a header row, a details row and one row per measured participant

Each processed period SHALL be written to the local sqlite audit database so that the payout is re-derivable from the database alone:

- A header row per period: for block signing the epoch id, start and end times, budget and a `disabled` flag; for issuance the expiration date, total budget, whitelist size, per-operator budget and a `disabled` flag.
- A details row per period carrying the period-wide measurement (total voting power at epoch start and block count, or approximate deposits), the amount spent, the transaction hash or the failure, and the `monitor_only` flag.
- One row per measured participant carrying the full working: for block signing the consensus address, operator account, whitelist flag, amount, voting power, voting-power share, signed blocks and signed ratio; for issuance the API endpoint, operator account, whitelist flag, banned flag, amount, issued count, issued share, `skipped_verification` and sample size.
- One additional row per ban, carrying the reason and the serialised evidence blob.

Amounts MUST be stored as their display strings (for example `670000000unym`). The header row MUST be written before any reward transaction for the period is sent, so a crash between broadcasting and persisting resumes past the period instead of paying it twice; a header with no matching details row is a period that was begun but whose settlement outcome is unknown (a reward transaction for it may or may not have been broadcast), which MUST be reported at startup as possibly unsettled and MUST NOT be replayed, since replaying it could double-pay. A disabled module's handler returns before measuring, so it writes nothing at all.

#### Scenario: Paid epoch is fully recorded
- **WHEN** an epoch is measured and paid
- **THEN** the epoch header, the details row with the transaction hash and spend, and one row per measured validator with its voting power, share, signed blocks and ratio are written

#### Scenario: A disabled module writes nothing
- **WHEN** a period elapses for a module that is disabled
- **THEN** the module's handler returns before measuring and no header, details or participant rows are written

#### Scenario: A ban is recorded with its evidence
- **WHEN** an issuer is caught cheating during an audit
- **THEN** its issuance row is flagged `banned` with a zero amount
- **AND** a ban row is written carrying the reason and the serialised evidence blob

### Requirement: A failure within a period is recorded against that period and the period is not retried

A measurement or settlement failure SHALL NOT abort the process. When results could not be computed, the rewarder MUST record a details row with sentinel measurements (`-1`) and the error text, and MUST advance the period marker. When a non-zero spend is recorded together with a failure to send, the rewarder MUST log `BROKEN INVARIANT` and skip writing the details row rather than record a contradiction. A failed settlement's error text is recorded in the `rewarding_error` column with `rewarding_tx` left NULL, so a reader never sees an error string where a transaction hash belongs.

#### Scenario: Failed measurement is recorded and the period advances
- **WHEN** a period's measurement returns an error
- **THEN** a details row with `-1` sentinels and the error text is written and the marker advances, with no retry

#### Scenario: Failed settlement stores its error in the error column
- **WHEN** the settlement transaction fails to broadcast
- **THEN** the recorded spend is zero and the error text appears in `rewarding_error` with `rewarding_tx` NULL

#### Scenario: Contradictory spend and failure is refused
- **WHEN** a period reports both a non-zero spend and a failure to send
- **THEN** `BROKEN INVARIANT` is logged and no details row is written for that period

### Requirement: The operator CLI provides initialisation, run, scraper backfill, identity regeneration and a dry-run audit

The binary SHALL expose exactly these subcommands, each accepting the shared override flags and an optional custom configuration path: `init`, `run`, `process-block`, `process-until`, `regenerate-identity`, `build-info` and `dry-run-check-issuer`.

- `init` MUST refuse to overwrite an existing configuration file unless `--force` is given, MUST create the config and data directories, MUST take the nyxd and websocket URLs from the first endpoint of the environment's network details (failing with `UnavailableWebsocketUrl` when no websocket URL is available), MUST generate a fresh ed25519 identity keypair, and MUST validate the configuration before saving it.
- `process-block` and `process-until` MUST drive the scraper directly to backfill history without rewarding; `process-until` MUST reject a start height greater than its stop height.
- `regenerate-identity` MUST refuse to overwrite existing key files unless `--unsafe-overwrite` is given, and MUST log a warning when it does overwrite them.
- `dry-run-check-issuer` MUST audit one issuer selected by operator account, API URL substring, node index or public key, MUST pin `full_verification_ratio` to 1.0 so the audit always runs, MUST refuse an already-banned issuer, and MUST NOT send any transaction or write any row. It MUST print the ban reason and the decoded evidence when cheating is detected, and otherwise the issued count, merkle root, sampled deposits and the issuer's claimed maximum response size.

#### Scenario: Init refuses to clobber an existing configuration
- **WHEN** `init` runs and the target configuration file already exists without `--force`
- **THEN** it fails with `ExistingConfig` and writes nothing

#### Scenario: Init with force also replaces the identity keypair
- **WHEN** `init --force` runs over an existing deployment
- **THEN** the configuration is overwritten and a fresh ed25519 keypair is written over the existing key files, without the `--unsafe-overwrite` guard that `regenerate-identity` applies

#### Scenario: Dry-run audits one issuer without paying
- **WHEN** `dry-run-check-issuer --signer <account> --expiration-date <date>` runs
- **THEN** that issuer is put through the full audit with the verification coin toss pinned to always audit
- **AND** the result is printed, no transaction is sent, and no database row is written

#### Scenario: Backfill is driven by the operator
- **WHEN** `process-until --start-height H` runs
- **THEN** the scraper processes the requested block range into its database and the process exits without rewarding

### Requirement: The configuration surface defines what every measurement means, and carries inert and unwired fields

The rewarder SHALL read its behaviour from a single TOML file with these fields and defaults: `upstream_nyxd` and `mnemonic`; `storage_paths` (scraper database, reward history database, ed25519 key files); `rewarding.daily_budget` (`24000000000 unym`) and `rewarding.ratios` (`0.67` / `0.33` / `0.0`); `block_signing` (`enabled = true`, `epoch_duration = 1h`, `monitor_only = false`, `whitelist`); `ticketbook_issuance` (`enabled = false`, `monitor_only = false`, `minimum_daily_ticketbooks = 200`, `min_validate_per_issuer = 100`, `sampling_rate = 0.05`, `full_verification_ratio = 1.0`, `whitelist`); and `nyxd_scraper` (`websocket_url`, `pruning`, `store_precommits = true`). Every field MAY be overridden at startup by the documented environment variables and CLI flags. The following are current-state facts of the surface:

- `rewarding.ratios.ticketbook_verification` MUST be part of a triple summing to 1.0 but has no implementation and no reader beyond that validation; there is no verification-rewarding module.
- The emitted `config.toml` template omits `ticketbook_issuance.minimum_daily_ticketbooks` and `nyxd_scraper.store_precommits`; both remain settable and fall back to their serde defaults.
- The `--daily-budget` flag and `NYM_VALIDATOR_REWARDER_DAILY_BUDGET` variable set `rewarding.daily_budget`.

#### Scenario: Defaults leave issuance off and block signing on
- **WHEN** a configuration omits both module sections
- **THEN** block signing is enabled with a 1 hour epoch and ticketbook issuance is disabled

#### Scenario: Whitelist can be supplied by environment variable
- **WHEN** `NYM_VALIDATOR_REWARDER_BLOCK_SIGNING_WHITELIST` carries a comma-separated list of consensus addresses
- **THEN** it replaces the configured block-signing whitelist for that process

#### Scenario: Epoch duration override is applied
- **WHEN** the process is started with `--epoch-duration 30m` while the file configures 1 hour
- **THEN** the flag parses and the block-signing epoch duration becomes 30 minutes

### Requirement: Pre-revamp rewarding tables are retained unread

The audit database SHALL retain the tables of the pre-revamp combined-rewarding scheme, renamed rather than dropped, as `combined_rewarding_epoch_v1`, `epoch_block_signing_v1` and `block_signing_reward_v1`. No code path reads or writes them; they exist so the historical record of payouts made under the single-epoch scheme survives migration. The credential-issuance tables of that era (`epoch_credential_issuance`, `malformed_credential`, `credential_issuance_reward`, `validated_deposit`, `double_signing_evidence`, `issuance_evidence`, `issuance_validation_failure`) were dropped in the same migration because the issuance measurement changed shape entirely.

#### Scenario: Legacy signing history survives migration
- **WHEN** a database created before the issuance revamp is migrated
- **THEN** its combined-epoch and block-signing rows remain readable under the `_v1` names
- **AND** no new rows are ever written to them

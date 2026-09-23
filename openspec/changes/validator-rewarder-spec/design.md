## Context

`nym-validator-rewarder` (v0.3.0, `nym-validator-rewarder/`) is a standalone, unattended binary holding a funded Nyx mnemonic. It pays two unrelated groups of operators for two unrelated kinds of work, from one shared daily budget: Nyx validators for signing blocks, and ecash DKG signers for issuing partial ticketbooks. It measures both itself rather than reading a score from anywhere else, and settles by sending an ordinary bank `send_multiple` transaction.

The two halves share only the process, the mnemonic, the sqlite audit database and the `daily_budget` line. Block signing runs on a configurable epoch ticker (default 1 hour) against a locally embedded chain scraper (`nyxd-scraper-sqlite`) that stores every block and pre-commit it sees over a websocket subscription. Ticketbook issuance runs once a day, shortly after midnight UTC, and performs an interactive, cryptographically verified audit against every current ecash signer's HTTP API. The two have separate `enabled` flags, separate `monitor_only` flags, separate whitelists in different address formats, separate resume markers and separate storage tables; starting with both disabled is a hard error and the only shared invariant.

The implementation has shipped and is documented only as Rust source plus scattered in-source comments and a startup-time error runbook. Critically, the measurement rules encode several non-obvious choices (a 20-block voting-power sampling window, a whitelist that doubles as the stake denominator, an expiration-date-keyed audit cohort, a per-issuer coin toss) and a handful of current-state slips that a reader would otherwise assume work correctly. This document captures the architecture and the rationale of the implementation as it stands on 2026-09-21. No behaviour change is proposed.

## Goals / Non-Goals

**Goals:**
- Fix what each of the two payouts actually measures, in normative terms, so a disputed payout can be adjudicated against a document rather than against `git blame`.
- Record the reward formulas exactly, including what happens to the unspent remainder of each budget, and the gating role each whitelist plays in its formula.
- Document the ticketbook issuance audit as a protocol: who is challenged, over what cohort, with what sampling, which answers are accepted, which answers are punished, and what the evidence of misbehaviour consists of.
- Document the operational envelope: the enable/`monitor_only` matrix, the pre-flight balance requirement, the daily minimum-deposit settlement gate, the failure posture (log and persist, never crash), the startup resync, and the manual recovery runbook the binary prints when resync fails.
- Record the configuration surface and its defaults, since every knob changes what a measurement means, and record which knobs are inert or silently ignored.
- Record the known current-state gaps as facts about the deployed binary, so they are neither mistaken for intended design nor silently "fixed" by a future reader who assumes the spec is aspirational.

**Non-Goals:**
- The internals of `nyxd-scraper-sqlite` (websocket subscription, block ingestion, pruning implementation). Its query surface is described only as the contract that defines "signed a block" and "blocks in an epoch".
- The internals of nym-api's ecash signer: how ticketbooks come to be issued, how the merkle tree is maintained, how deposits are validated, retention cleaning. The four audited endpoints and the retention cutoff are described as the dependency contract the audit is written against; their behaviour belongs to the nym-api ecash capability.
- The compact ecash scheme itself (`verify_partial_blind_signature`, attribute encoding) and the DKG ceremony that produces the signers' partial keys. The audit's use of them is in scope; their correctness is not.
- Nyx chain economics: where `daily_budget` comes from, whether the split between the two categories is correct, and who belongs on either whitelist. These are policy inputs, not behaviour.
- Any redesign of the flagged gaps. Ratifying current behaviour is the deliverable; fixes are follow-on changes.

## Decisions

### Decision 1: Three capabilities split along the module seam

**Choice.** The binary is specified as three capabilities: `validator-rewarder` (the shared core), `validator-rewarder-block-signing`, and `validator-rewarder-ticketbook-issuance`.

**Why.** The split mirrors a seam the code already has. The two measurement modules share no types, no storage tables, no cadence and no address format; they can be independently disabled, and each can be independently put in monitor-only mode. A single spec would force every reader interested in one payout to read the other, and every future change to one module to diff a document dominated by the other. The precedent for splitting one binary's surface across prefixed capabilities is `node-status-api-*`.

**Alternative considered.** One `validator-rewarder` capability, as with `network-monitor`. Rejected because the resulting spec would be roughly 600 lines with two mutually irrelevant halves. Also considered: two capabilities, folding the core into each. Rejected because the budget derivation, the balance pre-flight, the transaction-sending path, the CLI and the audit database are genuinely shared, and duplicating them would create two copies to keep in sync.

**Consequence.** The core capability is the smaller of the three and is mostly referenced by the other two. Requirements about "what a measurement means" live in the module capabilities; requirements about "when a measurement happens and how it is paid and recorded" live in the core.

### Decision 2: Block signing is scored from a privately scraped chain history

**Choice.** The rewarder embeds `nyxd-scraper-sqlite`, subscribes to a validator websocket, and stores every block and every pre-commit. A validator's signing score is derived entirely from that local database: `signed_blocks` is a `COUNT(*)` over stored `pre_commit` rows in the epoch's height range, and the epoch's block total is derived from the same table's height bounds.

**Why.** Cosmos SDK exposes only a rolling missed-block counter for slashing, over a window that does not align with the rewarder's epoch and that resets on jailing. Storing pre-commits gives an exact, re-checkable, per-epoch answer, and the same store supplies the voting power used to weight the reward.

**Alternative considered.** Querying the chain's slashing/signing-info module per epoch. Rejected as neither epoch-aligned nor auditable after the fact. Also considered: on-chain distribution of these rewards. Out of scope for this binary.

**Consequence.** Payout correctness depends on the scraper having complete coverage of the epoch, which makes pruning configuration load-bearing: `everything` pruning is rejected outright, and a `custom` strategy must keep at least `ceil(epoch_duration / 5s * 1.5)` recent blocks. A gap in coverage makes the epoch unpayable rather than under-paid, and recovery is manual (Decision 12). The database is also the reason the process is stateful and cannot simply be moved between hosts.

### Decision 3: Voting power is sampled once, from a 20-block window at the epoch start

**Choice.** For each validator known to the scraper, voting power is read from the first pre-commit found in `first_block..min(first_block + 20, last_block)`, scanning heights in order. A validator with no stored pre-commit anywhere in that window is skipped entirely and is absent from the epoch's results.

**Why.** A pre-commit carries its signer's voting power, so scanning a short window at the epoch boundary gives a stable "stake at the start of the epoch" figure without a separate staking query per validator per epoch, and without letting a validator change its weight mid-epoch by bonding or unbonding.

**Alternative considered.** Querying the staking module for each validator at the epoch's first height. Rejected as expensive and dependent on historical state the RPC node may have pruned. Also considered: averaging voting power across the epoch. Rejected as more complex with no clear benefit for hour-long epochs.

**Consequence.** The window is a hard eligibility gate, not just a weighting input. A validator that is down for the first 20 blocks (roughly 100 seconds) of an epoch earns nothing for that epoch even if it signs every subsequent block, and it does not contribute to the whitelisted voting-power total either, which inflates everyone else's share. The upper bound `min(first_block + 20, last_block)` means an epoch with fewer than 20 stored blocks samples a correspondingly shorter window, and the range is exclusive of its end.

### Decision 4: The block-signing whitelist gates both eligibility and the stake denominator

**Choice.** `block_signing.whitelist` holds `nvalcons1…` consensus addresses. Validators are measured regardless of membership, but a non-whitelisted validator gets `voting_power_ratio = 0`, a reward of zero, and, crucially, does not contribute to `total_vp`, the denominator of every other validator's voting-power share.

**Why.** The rewarder distributes a private budget, not chain inflation, so the operator chooses who is eligible. Excluding non-whitelisted stake from the denominator means the whitelisted set shares out the full budget among themselves rather than forfeiting the fraction of stake held by everyone else.

**Alternative considered.** Dividing by total network voting power and simply not paying non-whitelisted validators. Rejected because the budget would then be systematically under-spent by the non-whitelisted stake fraction.

**Consequence.** Adding or removing a whitelist entry changes every other whitelisted validator's payout. An empty whitelist is a hard startup error when the module is enabled. Non-whitelisted validators still appear in the audit database with their measured signing ratio and a zero amount, which makes the whitelist's effect auditable. Because `ratio_signed` is applied on top of the stake share, downtime under-spends the epoch budget: the remainder is simply never sent.

### Decision 5: Two budget shapes, one for each module

**Choice.** `daily_budget` is split by `ratios` (`block_signing`, `ticketbook_issuance`, `ticketbook_verification`, which must sum to exactly 1.0). Block signing then prorates its share by epoch length (`daily x ratio x epoch_duration / 24h`) and distributes it proportionally to `ratio_signed x voting_power_ratio`. Ticketbook issuance takes its share for the whole day and divides it into equal per-operator slices (`daily x ratio / whitelist_size`), each scaled by that operator's `issued_ratio`.

**Why.** The two activities have different notions of "a fair share". Block signing is inherently proportional to stake, so a single proportional pot is natural. Issuance is not: every signer is expected to issue the same ticketbooks as every other signer (clients collect a threshold of partial signatures), so equal slices with an individual completeness multiplier rewards each signer for keeping up, rather than making signers compete for a fixed pot.

**Alternative considered.** A single proportional pot for issuance, scaled by each signer's share of all partial ticketbooks issued. A surviving `TODO` in `types.rs` spells out the equal-slice shape as the intended replacement for that pot ("split reward into 1/whitelist size, then issued ratio is ratio of deposits in that time interval and thus your slice of the 1/whitelist"), and has since been implemented, but records no reason. The reading offered above (a proportional pot pays a signer more when its peers are down, so a signer benefits from its peers failing) is inferred from the formulas, not from the source, and is flagged for maintainer confirmation in the reviewer pass.

**Consequence.** Both budgets under-spend rather than redistribute. In block signing the shortfall is the missed-block fraction; in issuance it is the sum of unclaimed operator slices, including the slices of whitelisted operators that were absent, banned or not measured. `ticketbook_verification` exists only to make the ratios sum to 1.0: nothing reads it, and there is no verification-rewarding code.

### Decision 6: Two independent tickers, two independent resume markers

**Choice.** The main loop is a single biased `tokio::select!` over the shutdown signal, the scraper's cancellation token, a block-signing epoch ticker and a daily issuance ticker. Block signing advances a monotonic `Epoch { id, start_time, end_time }` persisted per epoch; issuance persists the last processed expiration date. Each module resumes from its own marker, and block signing additionally replays every already-finished epoch on startup before entering the loop.

**Why.** The two cadences are unrelated (hourly vs daily) and their natural keys are unrelated (an epoch id vs a calendar date). Independent markers let one module be disabled, re-enabled, or fall behind without corrupting the other's accounting. The scraper's cancellation token is in the same `select!` so that losing the chain subscription stops the process rather than letting it silently reward from a stalled database.

**Alternative considered.** A single unified rewarding epoch for both, which is what the pre-revamp schema did (the `_v1` tables). Rejected when issuance moved to a daily, expiration-date-keyed cohort that no hourly epoch can express.

**Consequence.** The first epoch of a fresh deployment begins at the next whole hour (`now + 1h`, truncated to the hour), so the process idles until that boundary before its first epoch even starts. The daily ticker fires at midnight plus two hours of leeway, and re-firing after a crash is guarded by comparing the resume marker against yesterday's date. The issuance path asserts that its computed "today" has hour zero, which is trivially true because `ecash_today()` is midnight-normalised, so the assertion documents an intent it cannot actually check.

### Decision 7: Issuance is audited by a signed, merkle-committed challenge-response

**Choice.** Rather than trusting a signer's self-reported count, the rewarder (a) asks for a signed commitment to the full deposit-id list plus a merkle root over the issued ticketbooks for one expiration date, (b) samples deposit ids from that committed list, (c) demands a merkle proof for exactly the sampled set, and (d) demands the raw blinded signatures and verifies each one against the signer's DKG partial verification key, re-deriving each merkle leaf hash and checking it sits at the committed index. Every request the rewarder sends is signed with its own ed25519 identity, and every response is signed by the issuer and carries the rewarder's original signed request back.

**Why.** The commitment is taken before the sample is drawn, so a signer cannot tailor its claimed set to the challenge, and the merkle root binds the whole set while only a sample is transferred. Signing in both directions makes an accusation self-contained: a ban record holds both public keys, the commitment, the challenge, and every signed response, so a third party can re-derive the verdict without trusting the rewarder. The rewarder signing its own request is what closes the "you asked for something else" defence, which is also why an echoed request whose signature no longer verifies is itself a bannable offence.

**Alternative considered.** Trusting the count endpoint (cheap, trivially inflatable). Also considered: downloading every issued ticketbook (correct but proportional to total issuance, and the signers cap response sizes). The commit-then-sample design gets probabilistic soundness at a bounded cost.

**Consequence.** The audit is only as strong as the sample, and it requires the rewarder to hold a long-lived identity key (regenerable with a dedicated subcommand, and required even when only block signing is enabled, because the keypair is loaded before the modules are constructed). A signer running an API too old to answer the commitment query is treated as having issued nothing rather than as cheating.

### Decision 8: The audited cohort is an expiration date, audited the day after it expires

**Choice.** Each daily run audits the cohort of ticketbooks whose expiration date is *yesterday*, not the ticketbooks issued yesterday.

**Why.** Expiration date is the only grouping key the signers' storage and merkle trees are organised by, so it is the only cohort a signer can commit to as a complete set. Waiting until the day after expiry guarantees the cohort is closed: no further ticketbook can be issued for a past expiration date, so the committed set cannot grow between the commitment and the challenge, and two signers' sets are comparable.

**Alternative considered.** Auditing by issuance date. Rejected because nothing on the issuer side is keyed that way, and an issuance-day cohort is still open at the time it would be measured.

**Consequence.** The measurement is deliberately late. With 7-day ticketbook validity, clients requesting the default expiration date (today + 6), and nym-api retaining issued ticketbooks for 2 days past expiry, the run audits work performed roughly a week earlier, in the last day before the signers are entitled to delete the evidence. The cadence, the validity period and the retention period are therefore a three-way constraint: lengthening retention is safe, shortening it below 2 days, or delaying the run by more than a day, makes the audit unanswerable through no fault of the signer.

### Decision 9: Auditing is probabilistic, per issuer and per ticketbook

**Choice.** Two independent randomisations. Per issuer, a weighted coin toss on `full_verification_ratio` (default 0.60) decides whether the full audit runs at all; a skipped issuer is still paid, and is recorded as `skipped_verification`. Per audited issuer, the challenge covers `max(min_validate_per_issuer, claimed_issued x sampling_rate)` deposits (defaults 10 and 0.01), drawn uniformly without replacement, or the whole set if the sample would cover it.

**Why.** Full verification of every signer every day costs bandwidth and CPU proportional to total issuance on both sides. Deterrence does not require certainty: a signer that cannot predict whether it will be audited, or which deposits will be sampled, must be able to answer correctly for all of them. The floor of 10 keeps small cohorts meaningfully checked, where 1% would round to zero.

**Alternative considered.** Auditing everyone at a lower sampling rate. Rejected because the per-issuer fixed costs (commitment retrieval, proof construction) dominate at small sample sizes, and because a skipped issuer still has to have been ready.

**Consequence.** On any given day roughly 40% of signers are paid on an unverified commitment, and a cheat confined to unsampled deposits goes undetected. A skipped issuer also contributes nothing to the deposit union (Decision 11) while still being paid from it. The `dry-run-check-issuer` subcommand exists to force a single issuer through the audit with the ratio pinned to 1.0, without paying or recording anything.

**Partial sampling is currently incompatible with the audit's own leaf-count check.** After verifying the merkle proof the rewarder also requires `proof.total_leaves() == sampled.len()`, but `total_leaves` is the size of the issuer's whole tree for the cohort, captured when the proof is generated, not the number of leaves the proof includes. The equality therefore holds only when the sample covers the entire cohort. Under the defaults (`min_validate_per_issuer = 10`, `sampling_rate = 0.01`) every audited issuer holding more than 10 ticketbooks in a cohort is banned on that check before any ticketbook data is requested, and because banned issuers contribute nothing to the deposit union, the day's `approximate_deposits` then stays near zero and the minimum-deposit gate withholds the settlement entirely. The audit is only self-consistent as written when the effective sample is the whole cohort, for example with `sampling_rate = 1.0`. The check the comment describes ("the same number of deposits as initially committed to", read as "the proof includes exactly the sampled leaves and nothing else") is not expressible through the proof type's public API, which exposes only `total_leaves()`, `contains_leaf_hash` and `contains_full_leaf`.

### Decision 10: Two-tier misbehaviour response, split at the challenge

**Choice.** Before the challenge, silence is unrewarded but not punished: a signer that does not answer the count query, does not answer the commitment query, or returns a commitment for the wrong expiration date is simply not paid. From the challenge onward, any answer that is not a correct one is treated as cheating and produces a ban with evidence: no response, a bad response signature, an echoed request whose rewarder signature no longer verifies, a self-declared `max_data_response_size` below `MINIMUM_TICKETBOOK_DATA_REQUEST_SIZE` (50), a wrong expiration date, a failing merkle proof, a proof whose leaf count differs from the sample size, a short or incomplete data batch, a deposit-id or expiration mismatch inside a returned ticketbook, a recomputed leaf absent from the proof, or a cryptographically invalid partial signature.

**Why.** The pre-challenge queries carry no element of chance: the rewarder asks every signer every day, so refusing to answer gains nothing and may simply mean an outdated or briefly unavailable API. Once a specific random sample has been named, however, silence and error are indistinguishable from evasion of an unfavourable draw, and are treated as such. The asymmetry is recorded in-source as deliberate.

**Alternative considered.** Banning on any failure, including the commitment query. Rejected as punishing version skew and transient unavailability.

**Consequence.** A ban zeroes that day's reward and is persisted with its evidence blob. Two paths currently fall through the split rather than landing on either side of it. A commitment for the wrong expiration date is logged as a mismatch and then kept: the audit continues against the requested date, so the issuer is usually banned at the challenge stage, but if the coin toss skipped its audit it is paid on the mismatched commitment, which the in-source comment ("we're just not going to reward them") does not describe. A commitment carrying no merkle root skips the challenge entirely on the assumption that nothing was issued, even though the issuer only reached that point by reporting a non-zero count; it is neither banned nor marked as skipped, its deposits widen the union, and it is paid on its claimed count. The persistent ban is **currently ineffective**: the insert passes the API URL where the `operator_account` column is expected and the account where the endpoint is expected, and the pre-ban check compares the `operator_account` column against an operator account, so a stored ban can never match on a later day. A caught signer is therefore re-audited and re-payable the next day, and a repeat offender is only unpaid on the days it is both audited and caught. The evidence is still recorded correctly enough to be read by a human, since both values are present, just in the wrong columns.

### Decision 11: The issuance denominator is the rewarder's own observed deposit union

**Choice.** `issued_ratio = claimed_issued / made_deposits`, where `made_deposits` is the set of distinct deposit ids the rewarder has collected from the commitments of issuers that completed the audit without being caught cheating, accumulated as issuers are checked in sequence. The same count, `approximate_deposits`, is what the daily settlement gate is tested against.

**Why.** There is no authoritative "number of ticketbooks that should have been issued today" available: deposits are chain events, but which ones a signer was asked to sign is not. Taking the union of what honest signers demonstrably did issue is a self-calibrating proxy for the day's true demand, and the name `approximate_deposits` records that it is an estimate.

**Alternative considered.** Counting deposits from the chain. Recorded in-source as the obvious source but not implemented; it would need the deposit-to-expiration-date mapping the rewarder does not track.

**Consequence.** The measure is order-dependent and unbounded. Issuers are checked sequentially, so the first issuer checked is divided by a union consisting of its own deposits and scores exactly 1.0, while later issuers are divided by a larger union. An issuer whose own deposits never entered the union (because its audit was skipped by the coin toss) can still be divided by a small early union and produce a ratio above 1.0, and the reward is `mul_floor` of the per-operator budget with no cap, so it can exceed that budget. Conversely, if the first issuer checked is skipped, the union is empty and its ratio is zero. Both are current-state facts of the deployed binary.

### Decision 12: Failure posture is log-and-persist, never crash, plus a manual recovery runbook

**Choice.** Within a period, every failure is caught, logged and recorded: a module that fails to compute results persists the failure for that epoch or date and advances its marker; a rewarding transaction that fails is recorded against the period; a per-validator or per-issuer problem is recorded on that row. The process exits only on shutdown, on loss of the scraper, or on a startup failure. Startup is where the strictness lives: both modules disabled, an empty whitelist for an enabled module, a missing identity key, an invalid pruning configuration for the epoch length, ratios that do not sum to 1.0, or a balance below seven days of budget all refuse to start. If the startup replay of finished epochs finds an epoch with no stored blocks, the process aborts and prints a five-step manual runbook (find the epoch's first height, run `process-until --start-height`, temporarily set `pruning.strategy = nothing`, restart until the missed rewards are sent, then re-enable pruning).

**Why.** An unattended payer that crashes mid-day stops paying everyone, which is worse than paying nobody for one epoch and recording why. Conversely, misconfiguration that would systematically mis-pay (no whitelist, wrong ratios, pruning that deletes the evidence) must be caught before any money moves, and an under-funded account must be caught before it starts paying partially.

**Alternative considered.** Automatic backfill of missing epochs, which the in-source comment acknowledges as the right answer and declines as "some serious refactoring". The runbook is the interim.

**Consequence.** Missed epochs need an operator. A failure recorded for an epoch is never retried, because the marker advances regardless, so an epoch recorded as failed is permanently unpaid. Recording of failures is itself imperfect: the error text of a failed rewarding transaction is written into the `rewarding_tx` column while `rewarding_error` is left NULL, so an operator reading the audit log sees a plausible-looking transaction field containing an error string.

### Decision 13: Persistence is an append-only audit log carrying the full working

**Choice.** Each period writes a header row (budget, whitelist size, per-operator budget, disabled flag), a details row (the epoch-wide measurement, the amount actually spent, the transaction hash or the error, and a `monitor_only` flag) and one row per measured participant carrying the complete working: for block signing the voting power, its share, the signed-block count and the signed ratio; for issuance the claimed count, the share, the sample size, and the skipped and banned flags. Bans additionally write a row holding the reason and the serialised evidence blob.

**Why.** Every payout must be re-derivable from the database alone, without the chain, the scraper or the signers, because that is what a payout dispute needs. Storing the inputs and the derived ratios, rather than just the amount, is what makes the arithmetic checkable.

**Alternative considered.** Storing only amounts and relying on the logs. Rejected as unauditable.

**Consequence.** Amounts are stored as their display strings (`"123unym"`) rather than as numbers, so aggregation requires parsing. The pre-revamp combined-rewarding tables survive as `combined_rewarding_epoch_v1`, `epoch_block_signing_v1` and `block_signing_reward_v1`, deliberately renamed rather than dropped so the historical record of the old single-epoch scheme is preserved; nothing reads them. A `monitor_only` value of true is also written when a settlement was withheld by the daily minimum-deposit gate, which conflates "configured not to pay" with "declined to pay today".

### Decision 14: Settlement is one bank transaction per module per period

**Choice.** Each module builds a list of `(recipient, [coin])` pairs, drops the zero amounts, and sends a single `send_multiple` with a human-readable memo. If the list is empty the module reports an error for the period instead of sending. A final guard rejects the whole transaction if any amount is zero or any coin list is empty.

**Why.** One transaction per period keeps fees and sequence-number handling trivial, makes the payout atomic (either everyone in the period is paid or nobody is) and gives a single hash to record in the audit row. Dropping zeros before sending is required because the chain rejects zero-amount sends, and the post-filter guard is a defence in depth against a formula change reintroducing them.

**Alternative considered.** One transaction per recipient. Rejected for fees, sequence contention and partial-payout states that the audit log cannot express.

**Consequence.** A single failing recipient (for example a malformed address) fails the whole period's payout. Block signing pays the operator account derived from the validator's `nvaloper…` operator address by swapping the bech32 prefix and recomputing the checksum, which assumes the operator and the reward recipient are the same key. Both memos are currently misleading: the block-signing memo is stamped with `last_processed_issuance_date`, which belongs to the other module and names a date unrelated to the epoch being paid, and the issuance memo reads the same field before it is advanced, so it names the *previous* cohort's expiration date rather than the one being paid.

## Risks / Trade-offs

- **The voting-power window is an eligibility cliff.** 100 seconds of downtime at the wrong moment costs a validator the whole epoch and inflates its peers' shares. → No mitigation in the current implementation; documented as a normative requirement so it is a known rule rather than a surprise.
- **Scraper coverage gaps make epochs permanently unpayable.** The marker advances past an epoch recorded as failed. → Mitigated by the pruning validation at startup, the startup replay, and the printed runbook; recovery is manual and must happen before pruning removes the evidence.
- **The issuance ratio is order-dependent and uncapped, so a day's issuance spend can exceed its budget.** → Currently unmitigated. Bounded in practice by the per-operator slice and by honest signers issuing near-identical sets, but not by construction.
- **The persistent ban never matches, so exclusion is not durable.** A caught signer is re-audited and re-payable the next day. → Partially mitigated by the audit being repeated daily: a persistent cheat is caught whenever it is sampled, and the evidence accumulates.
- **Roughly 40% of signers are paid unaudited each day, and sampling covers 1% (floor 10) of a cohort.** → Accepted trade-off; deterrence rests on unpredictability rather than coverage.
- **The leaf-count check bans honest issuers under partial sampling, and the resulting empty deposit union then withholds the whole day's settlement.** → Currently only avoidable by configuring the effective sample to cover the whole cohort; the module is disabled by default, which is what keeps this latent.
- **Two audit paths pay an unverified claim: a commitment for the wrong expiration date when the coin toss skips, and a commitment with no merkle root.** → Currently unmitigated; both are self-inconsistent responses that the flow neither punishes nor discounts.
- **The audit window is one day wide, against a 2-day retention period.** A run that is skipped, or a signer that is unreachable for a day, cannot be re-audited later because the data is legitimately deleted. → Mitigated only by the run's 2-hour post-midnight leeway.
- **A single funded mnemonic in the process' config file pays both categories.** → Mitigated by the seven-day balance pre-flight and by `monitor_only`, which exercises the full measurement path without sending anything.
- **Failure records are stored in misleading columns.** An error string in `rewarding_tx` can be misread as a hash, and `monitor_only` is set for withheld settlements. → Documented; fixes are follow-on changes.

## Migration Plan

Not applicable. This change adds documentation only: no code, no schema change, no configuration change, no deployment step. The specs describe the binary as deployed, so there is nothing to roll back.

## Open Questions

These are the judgment calls this change deliberately does not make. Each is recorded as current behaviour in the specs; resolving one either confirms the spec or opens a follow-on change.

1. **The swapped arguments that make the persistent issuer ban unmatchable (Decision 10).** Ratify as current state, or open a follow-on fix? A fix changes who gets paid, and would also want a decision on whether existing rows are migrated or the ban list is simply restarted.
2. **The merkle leaf-count check that bans honest issuers under partial sampling (Decision 9).** This is the one gap that makes the issuance module unusable as configured by default rather than merely imprecise, so it is the first candidate for a follow-on fix: compare against the number of leaves the proof actually includes (which needs a small accessor on `IssuedTicketbooksFullMerkleProof`), drop the check as redundant given proof verification plus the per-leaf index checks, or declare full-cohort sampling the intended configuration and remove the sampling knobs.
3. **The order-dependent, uncapped `issued_ratio` (Decision 11).** Should the ratio be capped at 1.0, should the deposit union be collected in a first pass before any reward is computed, or is the current behaviour acceptable given the per-operator slice?
4. **The two paths that pay an unverified claim (Decision 10): a commitment for the wrong expiration date when the coin toss skips the audit, and a commitment carrying no merkle root.** Should either be a ban, a zero reward, or left as is? The wrong-date case additionally needs a decision on whether the in-source comment or the code states the intent.
5. **The epoch block count is `last_block - first_block` while pre-commits are counted inclusively (Decision 2).** A validator that signs every block therefore scores `(n+1)/n`, and the in-code `debug_assert!(signed <= blocks)` would fire in a debug build. Off-by-one to fix, or intended as a rounding allowance? The same expression makes an epoch containing exactly one scraped block panic on a zero denominator.
6. **The 20-block voting-power window (Decision 3).** Intended as an eligibility gate, or only ever intended as a weighting sample? If the latter, a validator missing the window should presumably fall back to a staking query rather than be dropped.
7. **`ratios.ticketbook_verification` (Decision 5).** It must be part of a sum equal to 1.0 but has no implementation. Keep as a placeholder for a future verification-rewarding module, or remove it from the config surface?
8. **The `--epoch-duration` CLI flag is accepted and silently ignored** (`ConfigOverridableArgs` parses it, `override_config` never applies it), and `--epoch-budget` sets `rewarding.daily_budget` despite its name. Wire, rename, or remove?
9. **Both settlement memos name the wrong period (Decision 14).** The block-signing memo carries a ticketbook expiration date and the issuance memo carries the previous cohort's date. These are visible on chain; fix now or leave?
10. **The `_v1` legacy tables and the empty `rewarder/tasks/mod.rs` stub (Decision 13).** Keep the historical tables and the stub module, or clean both up?
11. **`monitor_only` is recorded as true when the daily minimum-deposit gate withholds a settlement (Decision 13).** Distinguish the two cases in the schema, or accept the conflation?

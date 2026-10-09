## 1. Source research (reverse-engineering inputs)

- [x] 1.1 Read the whole crate: `rewarder/mod.rs`, `rewarder/epoch.rs`, `rewarder/nyxd_client.rs`, `rewarder/helpers.rs`, `rewarder/block_signing/{mod,types}.rs`, `rewarder/ticketbook_issuance/{mod,verifier,types,helpers}.rs`, `rewarder/storage/{mod,manager}.rs`, `config/{mod,override,template,persistence}`, `cli/**`, `error.rs`, `migrations/*.sql`.
- [x] 1.2 Extract the configuration surface, defaults and validation rules, and cross-check them against the emitted `config.toml` template (found `minimum_daily_ticketbooks` and `store_precommits` omitted from the template).
- [x] 1.3 Trace the scraper query contract that defines the signing measurement (`get_first_block_height_after`, `get_last_block_height_before`, `get_blocks_between`, `get_signed_between`, `get_precommit`, `get_validators`) in `common/nyxd-scraper-sqlite/src/storage/`.
- [x] 1.4 Trace the four audited nym-api endpoints and their guards (`nym-api/src/ecash/api_routes/issued.rs`, `nym-api/src/ecash/state/mod.rs`, `state/local.rs`), including the retention cutoff and the maximum data response size.
- [x] 1.5 Read `common/ticketbooks-merkle/src/lib.rs` for leaf hashing, proof construction and the proof type's public API, and `common/ecash-time/src/lib.rs` plus `common/network-defaults/src/ecash.rs` for the day and expiration-date arithmetic and the two protocol constants.
- [x] 1.6 Verify the issuer discovery path against the DKG contract queries in `rewarder/nyxd_client.rs` (current dealers, verified shares, hard failure on a malformed share).
- [x] 1.7 Derive the budget arithmetic and confirm the worked examples (670 NYM hourly signing epoch, 7920 NYM daily issuance, 1980 NYM per operator at whitelist size 4, 168000 NYM balance floor, 1080-block custom pruning floor for a 1 hour epoch).
- [x] 1.8 Verify the current-state gaps against source rather than inference: swapped `insert_banned_ticketbook_issuer` arguments vs. the `banned_ticketbook_issuer` column order and the `is_banned` comparison; order-dependent and uncapped `issued_ratio`; `get_blocks_between` off-by-one against inclusive pre-commit counting; error text into `rewarding_tx`; both mis-stamped memos; `--epoch-duration` parsed but never applied; swapped whitelist log messages; trivially-true midnight assertion; inert `ticketbook_verification`; empty `rewarder/tasks/mod.rs`.
- [x] 1.9 Confirm the merkle leaf-count finding on both sides: `IssuedTicketbooksFullMerkleProof::total_leaves` is set from `inner.leaves_len()` at generation and nym-api generates proofs over the full daily tree, while the rewarder compares it against `sampled.len()` - so partial sampling bans honest issuers and, through the empty deposit union, withholds the day's settlement.

## 2. Author artifacts

- [x] 2.1 Write `proposal.md` (Why / What Changes / Capabilities / Impact) for three new capabilities, documentation only.
- [x] 2.2 Write `design.md` (Context / Goals-NonGoals / 14 Decisions / Risks / Migration Plan / Open Questions), recording for each decision the choice, the rationale, the alternative considered and the consequence, and flagging inferred rationale as inferred.
- [x] 2.3 Write `specs/validator-rewarder/spec.md`: module enablement, whitelists, identity key, config validation, pruning constraints, balance pre-flight, budget derivation, monitor-only, the main loop, epoch bookkeeping and resync, the recovery runbook, issuance-day resume, settlement, persistence, failure recording, the CLI, the configuration surface, and the retained `_v1` tables.
- [x] 2.4 Write `specs/validator-rewarder-block-signing/spec.md`: scrape-based measurement, epoch range resolution, the 20-block voting-power window, whitelist gating of the denominator, signed-block counting and the off-by-one, the reward formula, staking-detail resolution and its fallback, recipient derivation, total conversion guards, and per-validator recording.
- [x] 2.5 Write `specs/validator-rewarder-ticketbook-issuance/spec.md`: issuer discovery, the cohort rule and its retention constraint, the pre-ban check, the issuance-count gate, the commitment stage, the verification coin toss, sampling, the challenge and the leaf-count defect, batched data retrieval, the four per-ticketbook checks, the ban and evidence taxonomy, the rootless-commitment bypass, the reward formula with its order dependence, the minimum-deposit gate, and per-issuer recording.

## 3. Validate via openspec tooling

- [x] 3.1 Run `openspec validate validator-rewarder-spec --strict` and confirm it reports valid. **Valid.**
- [x] 3.2 Run `openspec show validator-rewarder-spec` and review the rendered output: check the requirement and scenario counts per capability and the section ordering. **Parsed: core 18 requirements / 48 scenarios, block signing 10 / 24, ticketbook issuance 15 / 41.**

## 4. Reviewer pass (before archiving)

- [ ] 4.1 Confirm the `proposal.md` "Why" matches operational understanding, in particular that the two modules are genuinely independent and that the whitelists are operator policy rather than protocol.
- [ ] 4.2 Walk `design.md` Decisions 1-14 and confirm each rationale matches team reasoning. Decision 5's "Alternative considered" carries an explicitly inferred rationale for the equal-slice issuance budget and needs either confirmation or correction.
- [ ] 4.3 Walk each spec requirement by requirement and, for every disagreement, decide whether the spec is wrong (edit the spec) or the implementation is wrong (open a follow-on change). The requirements that ratify a current-state gap are the ones to ratify or reject deliberately: the leaf-count check, the two unverified-claim payment paths, the unmatchable persistent ban, the order-dependent uncapped `issued_ratio`, the block-count off-by-one and its single-block panic, and the error text in `rewarding_tx`.
- [ ] 4.4 Resolve `design.md` Open Questions 1-11, recording each as document-and-keep or as a follow-on change. Question 2 (the leaf-count check) decides whether the issuance module is usable with partial sampling at all and should be settled first.
- [ ] 4.5 Confirm whether the deployed configuration runs the issuance module, and with what `sampling_rate` and `min_validate_per_issuer`, since that determines whether the leaf-count defect is latent or active. Record the answer in the design document.

## 5. Archive the change

- [ ] 5.1 Once reviewed and accepted, run `openspec archive validator-rewarder-spec` to promote the three delta specs into `openspec/specs/`.
- [ ] 5.2 Immediately run `git diff openspec/specs/` and account for every removed line, since archiving can silently drop scenarios.

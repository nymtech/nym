## ADDED Requirements

### Requirement: Block signing is measured from a locally scraped chain history, not from chain-native signing info

The block-signing module SHALL measure participation exclusively from an embedded `nyxd-scraper-sqlite` instance that subscribes to a configured validator websocket and stores every block and every pre-commit it observes. The module MUST NOT read the chain's slashing or signing-info modules for this purpose. The scraper MUST be started and MUST have completed its startup synchronisation before the first measurement, and its loss MUST stop the rewarder rather than allow measurement from a stalled database. The candidate set for an epoch MUST be every validator the scraper has ever recorded, not only the current validator set.

#### Scenario: Participation comes from stored pre-commits
- **WHEN** an epoch is measured
- **THEN** every validator's signing count is derived from `pre_commit` rows in the local scraper database
- **AND** no chain query for missed blocks or signing info is made

#### Scenario: Startup waits for the scraper to catch up
- **WHEN** the rewarder starts with block signing enabled
- **THEN** the scraper is started and its startup synchronisation is awaited before any epoch is processed

#### Scenario: Historical validators are still candidates
- **WHEN** a validator recorded by the scraper in the past is no longer in the active set
- **THEN** it is still enumerated as a candidate for the epoch, and is excluded only by the voting-power sampling gate

### Requirement: An epoch's measurement range is bounded by the first block after its start and the last block before its end

For an epoch `[start_time, end_time)` the module SHALL resolve `first_block` as the lowest-timestamp block strictly after `start_time` and `last_block` as the highest-timestamp block strictly before `end_time`, both from the scraper database. When either bound cannot be resolved the epoch MUST fail with `NoBlocksProcessedInEpoch` and no reward MUST be computed or sent for it. All per-validator counting MUST use this height range, so the measurement is defined by scraped coverage rather than by wall-clock time.

#### Scenario: Range is derived from stored block timestamps
- **WHEN** an epoch runs from 11:00 to 12:00 and the scraper holds blocks 100 (10:59), 101 (11:00:04) through 820 (11:59:57), and 821 (12:00:02)
- **THEN** `first_block` is 101 and `last_block` is 820

#### Scenario: Missing coverage voids the epoch
- **WHEN** the scraper holds no block after the epoch's start, or none before its end
- **THEN** the epoch fails with `NoBlocksProcessedInEpoch` and nothing is paid for it

### Requirement: Voting power is sampled from a 20-block window at the epoch start and acts as an eligibility gate

For each candidate validator the module SHALL take its voting power from the first pre-commit it finds while scanning heights in ascending order over `first_block .. min(first_block + 20, last_block)`, a range exclusive of its end. A validator with no stored pre-commit anywhere in that window MUST be logged as an error and excluded from the epoch entirely: it MUST NOT be rewarded, MUST NOT appear in the epoch's recorded results, and MUST NOT contribute to the whitelisted voting-power total. The sampled value MUST be used unchanged as that validator's voting power for the whole epoch, regardless of any bonding change during it.

#### Scenario: Presence in the window fixes the voting power for the epoch
- **WHEN** a validator's first pre-commit in the window is at `first_block + 3` with voting power 1200
- **THEN** its voting power for the epoch is 1200 even if its stake changes later in the epoch

#### Scenario: Absence from the window forfeits the epoch
- **WHEN** a validator has no pre-commit between `first_block` and `first_block + 19` but signs every later block in the epoch
- **THEN** an error is logged, it receives nothing, and it is absent from the epoch's recorded rows

#### Scenario: Short epochs sample a shorter window
- **WHEN** the epoch's stored range holds fewer than 20 blocks
- **THEN** the window ends at `last_block` and the scan covers only the blocks actually stored

### Requirement: The whitelist gates both the reward and the voting-power denominator

`block_signing.whitelist` holds `nvalcons1…` consensus addresses. Every candidate validator that passes the sampling gate MUST be measured and recorded, but a validator whose consensus address is not on the whitelist MUST receive `voting_power_ratio = Decimal::zero()`, MUST be paid nothing, and MUST NOT be added to `total_vp`, the denominator of every whitelisted validator's stake share. Consequently `total_vp` is the whitelisted voting power present in the sampling window, and adding or removing a whitelist entry changes every other whitelisted validator's payout. The current implementation's two log messages for this branch are swapped: a validator whose address parses but is absent from the whitelist is logged as "not a valid consensus address", while one whose address fails to parse is logged as "not on the whitelist". The behaviour is correct in both cases; only the messages are misleading.

#### Scenario: Non-whitelisted stake is excluded from the denominator
- **WHEN** three validators pass the sampling gate with voting powers 100, 100 and 100, and only two are whitelisted
- **THEN** `total_vp` is 200, each whitelisted validator's share is one half, and the third is recorded with a zero share and a zero amount

#### Scenario: Whitelist changes reprice everyone else
- **WHEN** a whitelisted validator holding one third of the whitelisted voting power is removed from the whitelist
- **THEN** the remaining whitelisted validators' shares rise to fill the whole budget in the next epoch

### Requirement: Signed blocks are counted as stored pre-commits over the epoch's inclusive height range

A validator's `signed_blocks` for an epoch SHALL be the number of `pre_commit` rows stored for its consensus address with `height >= first_block AND height <= last_block`. The epoch's block total SHALL be `last_block - first_block + 1`, counted inclusively on both ends so that it matches the pre-commit count. A validator that signed every block in range therefore scores exactly `n / n = 1.0`, the in-code `debug_assert!(signed <= blocks)` holds, and a perfect validator receives exactly its stake share of the budget. An epoch whose stored range collapses to a single height has a block total of 1, not 0, so the ratio computation never divides by zero.

#### Scenario: Perfect signing scores exactly one
- **WHEN** `first_block` is 101, `last_block` is 820, and a validator has a pre-commit at every height in that range
- **THEN** `signed_blocks` is 720, the epoch's block total is 720, and the signing ratio is exactly 1.0

#### Scenario: Partial signing scores proportionally
- **WHEN** a validator has pre-commits at 360 of the epoch's 720 heights
- **THEN** its signing ratio is `360 / 720`

#### Scenario: A single-block epoch has a block total of one
- **WHEN** an epoch's stored range resolves `first_block` equal to `last_block`
- **THEN** the block total is 1 and the ratio computation does not panic

### Requirement: A validator's reward is the epoch budget times its signing ratio times its stake share

For each measured validator the module SHALL compute `reward = floor(epoch_budget * ratio_signed * voting_power_ratio)` using `Decimal` arithmetic and `Uint128::mul_floor`, in the budget's denomination, and MUST return a zero amount for any validator that is not whitelisted without evaluating the product. Because the whitelisted shares sum to one, the epoch's total payout equals the budget only when every whitelisted validator signed every block; any downtime under-spends the budget and the remainder MUST NOT be redistributed. Zero amounts MUST be dropped before the settlement transaction is built.

#### Scenario: Full participation spends the whole epoch budget
- **WHEN** every whitelisted validator signed every block of the epoch
- **THEN** each receives `epoch_budget * voting_power_ratio` and the sum is the epoch budget, modulo flooring

#### Scenario: Downtime under-spends the budget
- **WHEN** a validator holding half the whitelisted voting power signed only half the epoch's blocks
- **THEN** it receives a quarter of the epoch budget and the unclaimed quarter stays in the rewarding account

#### Scenario: Non-whitelisted validators are dropped from the transaction
- **WHEN** a measured validator is not whitelisted
- **THEN** its amount is zero, it is recorded with that zero amount, and it is not a recipient of the settlement transaction

### Requirement: Validator staking details are resolved from the union of the historical valset and the live validator set

To label and pay a measured validator the module SHALL obtain staking details for the epoch from the union of the fully paginated live `validators` query and `historical_info(last_block)`, fetching the live set first and appending the historical entries so that the historical entry wins when both describe the same validator. Details MUST be keyed by the consensus address derived from each entry's consensus public key as `bech32(BECH32_CONSENSUS_ADDRESS_PREFIX, sha256(pubkey)[..20])`, and entries without a consensus public key MUST be skipped. A measured validator for which no staking entry can be found MUST be skipped with an error and left unpaid for the epoch, rather than voiding it; its voting power stays in the epoch's total, so its share is simply left unspent. A failure of the historical-info query MUST be logged as a warning and the live set used alone. The moniker MUST be taken from the entry's description, defaulting to `UNKNOWN MONIKER`.

#### Scenario: A jailed validator is resolved from the live set
- **WHEN** a validator that pre-committed inside the voting-power window was jailed before `last_block` and is absent from the historical valset
- **THEN** it is still resolved from the live staking store, which retains it for the unbonding period, and rewarded normally

#### Scenario: An unresolvable validator is skipped, not fatal
- **WHEN** a validator measured from the scraper has no entry in either the live set or the historical valset
- **THEN** it is skipped with an error and left unpaid, its stake share is left unspent, and every other validator is still rewarded for that epoch

### Requirement: Rewards are paid to the account derived from the validator's operator address

The recipient for a validator's block-signing reward SHALL be derived from its staking `operator_address` by re-encoding the same 20-byte payload under the account prefix, that is by swapping the `nvaloper…` bech32 prefix for `n1…` and recomputing the checksum. A payload that cannot be re-encoded MUST fail with `MalformedBech32Address`. The module MUST NOT accept a separately configured payout address; the whitelist controls eligibility only, and the recipient always follows from the validator's operator key.

#### Scenario: Operator address determines the recipient
- **WHEN** a rewarded validator's operator address is `nvaloper1abc…`
- **THEN** the reward is sent to `n1abc…` with the checksum recomputed for the account prefix

#### Scenario: Whitelist entries are consensus addresses, not payout addresses
- **WHEN** an operator is whitelisted by its `nvalcons1…` consensus address
- **THEN** the payment still goes to the account derived from its operator address, and the whitelist entry never names the recipient

### Requirement: Negative or unconvertible epoch totals fail the epoch

Before computing any ratio the module SHALL convert the epoch's total voting power and block count to unsigned values, failing the epoch with `NegativeTotalVotingPower` or `NegativeSignedBlocks` when either is negative. Per-validator voting power and signed-block counts that fail the same conversion MUST default to zero rather than fail the epoch. The computation assumes a whitelisted validator's sampled voting power is positive: a whitelisted validator with zero voting power and no other whitelisted validator would leave a zero denominator in the share computation.

#### Scenario: Negative epoch totals are refused
- **WHEN** the computed total voting power for an epoch is negative
- **THEN** the epoch fails with `NegativeTotalVotingPower` and nothing is paid

#### Scenario: A negative per-validator count degrades to zero
- **WHEN** a single validator's stored signed-block count fails conversion to an unsigned value
- **THEN** that validator's count is treated as zero and the rest of the epoch is computed normally

### Requirement: Every measured validator is recorded with the full working, paid or not

For each epoch the module SHALL persist, per measured validator, its consensus address, the derived operator account, its whitelist flag, its reward amount, its sampled voting power, its voting-power share, its signed-block count and its signing ratio, and MUST log the moniker, amount, recipient and whitelist flag for each at computation time. Validators excluded by the sampling gate are absent from these rows; validators excluded only by the whitelist MUST be present with a zero share and a zero amount, which is what makes the whitelist's effect auditable from the database alone.

#### Scenario: Unpaid but measured validators are still recorded
- **WHEN** an epoch is recorded and a measured validator is not whitelisted
- **THEN** a row exists for it carrying its signing ratio and voting power with a zero share and a zero amount

#### Scenario: Per-validator working is re-derivable from the database
- **WHEN** an operator disputes an epoch's payout
- **THEN** the epoch's block total, total voting power, and the validator's voting power, share, signed blocks and ratio are all available in the audit database without re-querying the chain

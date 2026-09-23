## ADDED Requirements

### Requirement: Issuers are discovered from the DKG contract, and contract inconsistency fails the whole run

The set of ticketbook issuers SHALL be derived from chain state, never from configuration: the module MUST read the current DKG epoch, take all current dealers, take all verification key shares for that epoch, and keep only those entries whose dealer is still current and whose share is marked `verified`. For each kept entry it MUST resolve the issuer's partial verification key, its `n1…` operator account, its announced API URL and its ed25519 identity, producing one issuer record carrying also the dealer's assigned index. An unverified share MUST be skipped with a warning. An unparseable announce URL, an unparseable ed25519 identity or an HTTP client that cannot be constructed MUST skip that issuer with a warning. A verification key share that cannot be decoded MUST instead fail the entire run with `MalformedPartialVerificationKey`, because contract data is expected to be correct for every signer and no one should be rewarded while it is not.

#### Scenario: Only verified current dealers are audited
- **WHEN** the contract holds five verification key shares for the current epoch and one is not marked `verified`
- **THEN** four issuers are audited and the unverified one is skipped with a warning

#### Scenario: Malformed contract data blocks the whole day
- **WHEN** one issuer's verification key share cannot be decoded
- **THEN** the run fails with `MalformedPartialVerificationKey` and no issuer is rewarded for that day

#### Scenario: An unreachable announce address skips one issuer only
- **WHEN** an issuer's announced API URL does not parse
- **THEN** that issuer is skipped with a warning and the remaining issuers are audited normally

### Requirement: Each daily run audits the cohort of a single expiration date, the day after it expired

The module SHALL measure issuance per expiration-date cohort and MUST audit the cohort whose expiration date is the day before the current ecash day. The choice is load-bearing and is constrained from both sides: a cohort keyed by expiration date is closed once that date has passed, so the committed set cannot grow between the commitment and the challenge, and nym-api retains issued ticketbooks for only `issued_ticketbooks_retention_period_days` (default 2) days beyond their expiration date, so a cohort audited later than one day after expiry can become legitimately unanswerable. With `TICKETBOOK_VALIDITY_DAYS` of 7 and clients requesting the default expiration date of `today + 6`, the audited work was therefore performed approximately a week before the audit. The signers MUST also reject a requested expiration date earlier than their retention cutoff or later than their default expiration date, so requesting any other cohort is not available to the rewarder.

#### Scenario: Yesterday's expiration date is the cohort
- **WHEN** the daily run executes on 2026-09-21
- **THEN** it audits the ticketbooks whose expiration date is 2026-09-20, which were issued around 2026-09-14

#### Scenario: Retention bounds the audit window
- **WHEN** a daily run is missed and the same cohort would be audited two days after expiry
- **THEN** the signers are entitled to have deleted the data and the cohort cannot be audited at all

### Requirement: Previously banned issuers are skipped without being tested

Before auditing, the module SHALL load the persisted ban list and MUST skip any issuer on it, recording it as pre-banned with a zero reward, a zero sample size and a zero issued count, without making any request to it. The persistent ban is **currently ineffective**: the ban row is written with the issuer's API URL in the `operator_account` column and its operator account in the `api_endpoint` column, while the load reads the `operator_account` column and the comparison is made against an operator account. No stored ban can therefore ever match, so a caught issuer is re-audited and re-payable on every subsequent day, and the same mismatch makes the `dry-run-check-issuer` refusal for banned issuers equally unreachable. The evidence itself is stored intact, so the ban record remains usable by a human reading the database.

#### Scenario: A pre-banned issuer would be skipped untested
- **WHEN** an issuer's operator account appears in the loaded ban list
- **THEN** it is recorded as pre-banned with a zero amount and no request is sent to it

#### Scenario: Yesterday's ban does not carry over today
- **WHEN** an issuer was banned with evidence during the previous day's run
- **THEN** today's run does not match it against the ban list and audits it again as if it had never been banned

### Requirement: An issuer that claims no issuance is dropped from the run entirely

The module SHALL first ask each issuer for its issued-ticketbook count for the cohort. An issuer that reports zero, or whose count query fails for any reason, MUST be excluded from the run completely: it MUST NOT be challenged, MUST NOT be rewarded, and MUST NOT have a row written for it. A failed count query MUST be logged as the issuer not supporting the queries required for rewarding, so an unreachable issuer and a genuinely idle one are indistinguishable in the record. The count query doubles as the version probe that decides whether the remaining audit endpoints can be expected at all.

#### Scenario: Idle issuer is absent from the results
- **WHEN** an issuer reports zero issued ticketbooks for the cohort
- **THEN** it is not challenged, not rewarded, and no issuance row is written for it

#### Scenario: Unreachable issuer is treated as idle
- **WHEN** an issuer's count endpoint returns an error or is not implemented
- **THEN** a warning is logged, the issuer is dropped from the run, and no ban is recorded

### Requirement: The issued-set commitment is obtained and signature-checked before any challenge is issued

For each remaining issuer the module SHALL request a signed commitment for the cohort consisting of the full list of `(deposit_id, merkle_index)` pairs and the merkle root over the issued ticketbooks, and MUST store the response as evidence material whether or not it is valid. A transport failure MUST leave the issuer unrewarded without a ban, on the explicit grounds that every issuer is asked every day and refusing carries no element of chance. A response whose signature does not verify against the issuer's ed25519 identity MUST be a ban. A response whose expiration date does not match the requested one MUST currently only be logged as a mismatch: the commitment is retained and the audit continues against the requested date, so such an issuer is typically banned later at the challenge stage, and is paid on the mismatched commitment whenever the verification coin toss skips its audit. The in-source comment claiming such an issuer is "just not going to be rewarded" does not correspond to any code path.

#### Scenario: Silence before the challenge is unrewarded but unpunished
- **WHEN** an issuer does not answer the commitment request
- **THEN** it is recorded with a zero reward and no ban is created

#### Scenario: A forged commitment signature is a ban
- **WHEN** the commitment response's signature does not verify against the issuer's identity key
- **THEN** the issuer is banned with basic evidence naming the bad signature on the issued ticketbooks for that date

#### Scenario: A wrong-date commitment is currently payable
- **WHEN** an issuer returns a commitment for an expiration date other than the requested one and the verification coin toss skips its audit
- **THEN** only a mismatch warning is logged and the issuer is rewarded on the claimed count of that mismatched commitment

### Requirement: A weighted coin toss decides whether an issuer is audited at all

After obtaining the commitment the module SHALL draw a weighted boolean with probability `full_verification_ratio` (default 0.60) and, when it comes up false, MUST mark the issuer `skipped_verification` and stop its audit there. A skipped issuer MUST still be rewarded, on the strength of its unverified commitment, and MUST NOT contribute its deposits to the observed deposit union. The toss MUST be drawn independently per issuer. `dry-run-check-issuer` MUST pin the ratio to 1.0 so a manual check always audits.

#### Scenario: Skipped issuer is paid unaudited
- **WHEN** the coin toss for an issuer comes up false
- **THEN** no challenge is issued, the issuer is recorded as `skipped_verification`, and it is still rewarded from its claimed count

#### Scenario: Skipped issuer does not widen the deposit union
- **WHEN** an issuer's audit is skipped
- **THEN** its committed deposit ids are not added to the observed deposit union, although its claimed count is still divided by that union

#### Scenario: Roughly two fifths of issuers go unaudited daily
- **WHEN** the default `full_verification_ratio` of 0.60 is in force
- **THEN** each issuer has a 40% chance per day of being paid without any cryptographic verification

### Requirement: The challenge sample is drawn uniformly from the committed deposit ids

For an audited issuer the module SHALL compute a desired sample size of `max(min_validate_per_issuer, floor(claimed_issued * sampling_rate))` (defaults 10 and 0.01) and MUST draw that many deposit ids uniformly without replacement from the committed list, or take the entire committed list when the desired size is greater than or equal to it. The sample MUST be drawn only after the commitment has been received, so the issuer cannot know which deposits will be challenged when it commits.

#### Scenario: Small cohorts are sampled in full
- **WHEN** an issuer committed to 8 deposits and `min_validate_per_issuer` is 10
- **THEN** all 8 deposits are challenged

#### Scenario: Large cohorts are sampled at the configured rate
- **WHEN** an issuer committed to 5000 deposits, `sampling_rate` is 0.01 and `min_validate_per_issuer` is 10
- **THEN** 50 deposits are drawn uniformly at random

#### Scenario: The sample is unpredictable at commitment time
- **WHEN** an issuer produces its commitment
- **THEN** the deposit ids to be challenged have not yet been chosen, so the commitment must be correct for every deposit in it

### Requirement: The challenge commitment must carry a merkle proof for the sampled deposits, and the leaf-count check currently demands a full-cohort sample

The module SHALL send a signed challenge naming the cohort and the sampled deposit ids, and MUST require the response to satisfy all of: a valid signature by the issuer, a self-declared `max_data_response_size` of at least `MINIMUM_TICKETBOOK_DATA_REQUEST_SIZE` (50), an intact rewarder signature on the echoed original request, the requested expiration date, a merkle proof that verifies against the root committed earlier, and a proof whose `total_leaves()` equals the number of sampled deposits. Each failure MUST be a ban.

The last of those checks is a **current-state defect**. `total_leaves()` is the size of the issuer's whole merkle tree for the cohort, recorded when the proof is generated, not the number of leaves included in the proof. The check therefore passes only when the sample covers the entire cohort, and bans an honest issuer whenever it does not. With the default `min_validate_per_issuer = 10` and `sampling_rate = 0.01`, every audited issuer holding more than 10 ticketbooks in a cohort is banned on the leaf-count check, before any ticketbook data is requested. The practical consequences are that the audit is only self-consistent when the effective sample covers the whole cohort (for example `sampling_rate = 1.0`), and that under partial sampling the observed deposit union stays near zero, which in turn withholds the day's settlement through the minimum-deposit gate. The apparently intended check - that the proof includes exactly the sampled leaves and no others - is not expressible through the proof's public API, which exposes only `total_leaves()`, `contains_leaf_hash` and `contains_full_leaf`.

#### Scenario: Full-cohort sample satisfies the leaf-count check
- **WHEN** an issuer committed to 8 deposits, all 8 are sampled, and it returns a valid proof
- **THEN** `total_leaves()` is 8, equals the sample size, and the audit proceeds to data retrieval

#### Scenario: Partial sample bans an honest issuer
- **WHEN** an honest issuer committed to 500 deposits, 10 are sampled, and it returns a valid merkle proof for those 10
- **THEN** the proof verifies against the committed root but `total_leaves()` is 500, the leaf-count check fails, and the issuer is banned for an invalid merkle proof

#### Scenario: A degenerate declared batch size is a ban
- **WHEN** an issuer declares a `max_data_response_size` below 50
- **THEN** it is banned for a maximum data request size below the protocol minimum

#### Scenario: A tampered echo of the rewarder's request is a ban
- **WHEN** the echoed original request no longer verifies against the rewarder's public key
- **THEN** the issuer is banned for tampering with the request, and the tampered request is included in the evidence

### Requirement: Ticketbook data is retrieved over signed requests in issuer-declared batches, and must be complete

The module SHALL retrieve the raw issued-ticketbook data for the sampled deposits in batches of the issuer's declared `max_data_response_size`, each batch requested with a rewarder-signed body naming the cohort and that batch's deposit ids. For each batch response it MUST require a valid issuer signature, an intact rewarder signature on the echoed request, a returned map whose length equals the requested batch length, and the presence of every requested deposit id. Each failure MUST be a ban, including a transport failure, on the grounds that by this point the issuer knows which deposits were sampled and silence is indistinguishable from evading an unfavourable draw.

#### Scenario: Data is retrieved in declared batches
- **WHEN** 250 deposits were sampled and the issuer declared a maximum response size of 100
- **THEN** three signed data requests are made and their responses are accumulated

#### Scenario: A missing deposit in the response is a ban
- **WHEN** a batch response omits one of the requested deposit ids
- **THEN** the issuer is banned for an incomplete response naming that deposit id, with the full response retained as evidence

#### Scenario: Refusing to answer after the challenge is a ban
- **WHEN** a data request fails at the transport level
- **THEN** the issuer is banned for not responding for the cohort, with the error text recorded in the evidence

### Requirement: Every returned partial ticketbook is verified four ways

For each returned partial ticketbook the module SHALL verify, in order: that its `deposit_id` matches the key it was returned under; that its `expiration_date` equals the audited cohort; that the leaf recomputed from it as `sha256(deposit_id || epoch_id || blinded_partial_credential || private_attribute_commitments || expiration_julian_day || ticketbook_type)` is present in the merkle proof at exactly the `merkle_index` the issuer committed to for that deposit; and that the blinded signature cryptographically verifies under `verify_partial_blind_signature` against the issuer's partial verification key, with public attributes consisting of the expiration date scalar and the ticketbook type scalar. Any failure MUST be a ban. Success MUST be logged with the number of ticketbooks verified.

#### Scenario: A valid sample is accepted
- **WHEN** every returned partial ticketbook matches its deposit id, cohort, committed merkle index and partial key
- **THEN** the audit passes and the issuer's deposits are added to the observed deposit union

#### Scenario: A substituted ticketbook is caught by the merkle index
- **WHEN** a returned ticketbook hashes to a leaf that is not in the proof at its committed index
- **THEN** the issuer is banned for a missing partial ticketbook merkle leaf, with the expected leaf recorded as evidence

#### Scenario: A cryptographically invalid partial is a ban
- **WHEN** a returned blinded signature fails verification against the issuer's partial verification key
- **THEN** the issuer is banned for a cryptographically malformed ticketbook

#### Scenario: An inconsistent expiration date inside the data is a ban
- **WHEN** a returned ticketbook's expiration date differs from the audited cohort
- **THEN** the issuer is banned for an inconsistent partial ticketbook expiration date, recording both the claimed and actual values

### Requirement: A ban carries a reason and self-contained evidence, and zeroes that day's reward

Every ban SHALL record a human-readable reason and a serialised evidence package containing the rewarder's public key, the issuer's public key, the issued-set commitment, the requested challenge deposit ids, the challenge commitment response and every ticketbook data response received, plus a per-case context object (a requested/received mismatch, a tampered original request, a claimed/actual mismatch, an expected merkle leaf, or an error string). Because every request is signed by the rewarder and every response by the issuer, the package MUST be verifiable by a third party without trusting the rewarder. A banned issuer MUST receive a zero reward for that day regardless of its claimed count or whitelist status, MUST stop being probed as soon as it is banned, and MUST NOT contribute its deposits to the observed deposit union.

#### Scenario: Evidence is self-contained
- **WHEN** an issuer is banned at any stage
- **THEN** the stored evidence contains both public keys and every signed message exchanged, so the verdict can be re-derived independently

#### Scenario: A ban stops the audit immediately
- **WHEN** an issuer is banned at the challenge stage
- **THEN** no ticketbook data is requested from it and no cryptographic verification is attempted

#### Scenario: A banned issuer earns nothing that day
- **WHEN** a whitelisted issuer is banned during the run
- **THEN** its recorded amount is zero and it is not a recipient of the day's transaction

### Requirement: A commitment without a merkle root skips the challenge and is still rewarded

When an issuer's commitment carries no merkle root the module SHALL skip the challenge entirely, on the assumption that a missing root means nothing was issued. Because an issuer only reaches this point after reporting a non-zero count, the combination is self-inconsistent, and it is currently neither punished nor unrewarded: no challenge is issued, no data is requested, no verification runs, the issuer is not marked as skipped, its claimed deposits are added to the observed deposit union, and it is paid on its claimed count.

#### Scenario: Rootless commitment bypasses verification and is paid
- **WHEN** an issuer reports a non-zero count and then returns a signed commitment listing deposits with a null merkle root
- **THEN** no challenge or data request is made, no ban is recorded, its deposits widen the observed deposit union, and it is rewarded on its claimed count

### Requirement: An issuer's reward is its per-operator budget times its share of the observed deposit union

For each recorded issuer the module SHALL compute `issued_ratio = claimed_issued / |observed_deposit_union|`, where the union is the set of distinct deposit ids collected from the commitments of issuers that completed their audit without being caught cheating, and `reward = floor(per_operator_budget * issued_ratio)`, zero if the issuer is not whitelisted or is banned. The union size MUST also be recorded for the day as `approximate_deposits`, and a union of size zero MUST yield a ratio of zero for everyone.

The measure is order-dependent and uncapped, and both are current-state facts. Issuers are audited sequentially and each issuer's ratio is computed against the union as it stands immediately after its own audit, so the first issuer audited is divided by a union consisting only of its own deposits and scores exactly 1.0. An issuer whose deposits never entered the union (because its audit was skipped or it was banned) can be divided by a smaller earlier union and produce a ratio above 1.0, which `mul_floor` does not clamp, so its reward can exceed its per-operator budget and the day's total spend can exceed the issuance budget. If the first issuer audited is skipped, it is divided by an empty union and earns nothing.

#### Scenario: An issuer that kept up earns its full slice
- **WHEN** an audited issuer's committed set equals the observed deposit union
- **THEN** its ratio is 1.0 and it receives the whole per-operator budget

#### Scenario: A lagging issuer earns proportionally less
- **WHEN** an issuer committed to 800 of the 1000 deposits in the observed union
- **THEN** its ratio is 0.8 and it receives 80% of the per-operator budget

#### Scenario: Audit order changes payouts
- **WHEN** two issuers committed to overlapping sets of 900 and 1000 deposits and both are audited
- **THEN** the one audited first is divided by the union as it stood after its own audit and the other by the larger union, so their ratios depend on the order in which the issuers were enumerated

#### Scenario: A skipped early issuer can be overpaid
- **WHEN** the first audited issuer contributes 800 deposits to the union and the next issuer's audit is skipped while it claims 1000
- **THEN** the skipped issuer's ratio is 1.25 and its reward is 125% of the per-operator budget, uncapped

### Requirement: The day's settlement is withheld unless the observed deposit union reaches the configured minimum

Before sending the day's transaction the module SHALL compare `approximate_deposits` against `ticketbook_issuance.minimum_daily_ticketbooks` (default 200) and MUST NOT send any transaction when the union is smaller, or when the day's results could not be computed. Reward amounts MUST still be computed, recorded per issuer, and the day MUST still be recorded; the withheld settlement MUST be recorded with a zero spend and no transaction hash, which the storage layer currently represents with the same `monitor_only` flag used for a configured monitoring-only deployment.

#### Scenario: A quiet day pays nobody
- **WHEN** the observed deposit union holds 150 deposits and the minimum is 200
- **THEN** no transaction is sent, per-issuer amounts are still recorded, and the day is recorded with a zero spend

#### Scenario: Withheld settlement is indistinguishable from monitoring mode
- **WHEN** a day's settlement is withheld by the minimum-deposit gate
- **THEN** the day's details row is flagged `monitor_only`, the same flag a deliberately monitoring-only deployment writes

### Requirement: Every recorded issuer carries the full working, and bans carry their evidence

For each day the module SHALL persist, per recorded issuer, its API endpoint, operator account, whitelist flag, banned flag (set when the issuer was pre-banned or banned during the run), reward amount, claimed issued count, issued share as a float, `skipped_verification` flag and sample size, and MUST log per issuer its endpoint, account, amount, whitelist flag and issued count at computation time. Issuers dropped for claiming no issuance MUST be absent from these rows. Each ban MUST additionally write a row carrying the endpoint, account, the time of the ban, the audited expiration date, the reason and the serialised evidence blob.

#### Scenario: A skipped audit is visible in the record
- **WHEN** an issuer was paid without verification
- **THEN** its row carries `skipped_verification = true` and a sample size of zero alongside its amount

#### Scenario: A ban is fully reconstructible from the database
- **WHEN** an issuer was banned during a run
- **THEN** its issuance row is flagged banned with a zero amount, and a ban row holds the reason and the evidence needed to re-derive the verdict

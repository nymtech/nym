/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */


-- explicitly mark end of "old" combined rewarding with the `_v1` suffix
-- (as a result, we have to recreate bunch of tables due to foreign key constraints)
ALTER TABLE rewarding_epoch
    RENAME TO combined_rewarding_epoch_v1;
ALTER TABLE epoch_block_signing
    RENAME TO epoch_block_signing_v1;
ALTER TABLE block_signing_reward
    RENAME TO block_signing_reward_v1;


CREATE TABLE block_signing_rewarding_epoch
(
    id         INTEGER                     NOT NULL PRIMARY KEY,
    start_time TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    end_time   TIMESTAMP WITHOUT TIME ZONE NOT NULL,

    -- rewarding budget allocated to this block signing rewarding epoch
    budget     TEXT                        NOT NULL,

    -- indicates whether block signing rewarding/monitoring module is disabled
    disabled   BOOLEAN                     NOT NULL
);

CREATE TABLE block_signing_rewarding_details
(
    rewarding_epoch_id                INTEGER NOT NULL REFERENCES block_signing_rewarding_epoch (id),

    -- total voting power at the start of the epoch used for determining relative rewards
    total_voting_power_at_epoch_start INTEGER NOT NULL,

    -- total number of blocks processed during this rewarding epoch
    num_blocks                        INTEGER NOT NULL,

    -- the actual amount spent (decreased by missing blocks, etc.)
    spent                             TEXT    NOT NULL,

    -- if successful, the transaction hash of the rewarding transaction
    rewarding_tx                      TEXT,

    -- if unsuccessful, the error indicating why the rewards were not sent out
    rewarding_error                   TEXT,

    -- indicates whether this instance is running in 'monitor only' mode where it's not expected to be sending any transactions
    monitor_only                      BOOLEAN NOT NULL
);

CREATE TABLE block_signing_reward
(
    rewarding_epoch_id          INTEGER NOT NULL REFERENCES block_signing_rewarding_epoch (id),
    validator_consensus_address TEXT    NOT NULL,
    operator_account            TEXT    NOT NULL,
    whitelisted                 BOOLEAN NOT NULL,
    amount                      TEXT    NOT NULL,
    voting_power                BIGINT  NOT NULL,
    voting_power_share          TEXT    NOT NULL,
    signed_blocks               INTEGER NOT NULL,
    signed_blocks_percent       TEXT    NOT NULL,

    UNIQUE (rewarding_epoch_id, operator_account)
);


-- recreate tables for issuance rewarding as the mechanisms/epochs/etc for verification changed
DROP TABLE epoch_credential_issuance;
DROP TABLE malformed_credential;
DROP TABLE credential_issuance_reward;
DROP TABLE validated_deposit;
DROP TABLE double_signing_evidence;
DROP TABLE issuance_evidence;
DROP TABLE issuance_validation_failure;



CREATE TABLE ticketbook_issuance_epoch
(
    expiration_date     DATE    NOT NULL PRIMARY KEY,

    -- rewarding budget allocated to this ticketbook issuance epoch
    total_budget        TEXT    NOT NULL,

    whitelist_size      INTEGER NOT NULL,

    -- rewarding budget allocated for a single operator based on total budget and whitelist size
    budget_per_operator TEXT    NOT NULL,

    -- indicates whether block signing rewarding/monitoring module is disabled
    disabled            BOOLEAN NOT NULL
);

CREATE TABLE ticketbook_issuance_rewarding_details
(
    ticketbook_expiration_date DATE    NOT NULL REFERENCES ticketbook_issuance_epoch (expiration_date),

    -- approximate numbers of total deposits made with the particular expiration date
    approximate_deposits       INTEGER NOT NULL,

    -- the actual amount spent (decreased by not issuing every available ticketbook, etc. it's not expected everyone will ever get 100%)
    spent                      TEXT    NOT NULL,

    -- if successful, the transaction hash of the rewarding transaction
    rewarding_tx               TEXT,

    -- if unsuccessful, the error indicating why the rewards were not sent out
    rewarding_error            TEXT,

    -- indicates whether this instance is running in 'monitor only' mode where it's not expected to be sending any transactions
    monitor_only               BOOLEAN NOT NULL
);


CREATE TABLE ticketbook_issuance_reward
(
    ticketbook_expiration_date  DATE    NOT NULL REFERENCES ticketbook_issuance_epoch (expiration_date),
    api_endpoint                TEXT    NOT NULL,
    operator_account            TEXT    NOT NULL,
    whitelisted                 BOOLEAN NOT NULL,
    banned                      BOOLEAN NOT NULL,
    amount                      TEXT    NOT NULL,
    issued_partial_ticketbooks  INTEGER NOT NULL,
    share_of_issued_ticketbooks FLOAT   NOT NULL,
    skipped_verification        BOOLEAN NOT NULL,
    subsample_size              INTEGER NOT NULL,

    UNIQUE (ticketbook_expiration_date, operator_account)
);

CREATE TABLE banned_ticketbook_issuer
(
    operator_account                      TEXT PRIMARY KEY            NOT NULL,
    api_endpoint                          TEXT                        NOT NULL,
    banned_on                             TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    associated_ticketbook_expiration_date DATE                        NOT NULL REFERENCES ticketbook_issuance_epoch (expiration_date),
    reason                                TEXT                        NOT NULL,
    evidence                              BLOB                        NOT NULL
)
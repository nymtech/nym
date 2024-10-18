/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */


-- explicitly mark end of "old" combined rewarding
-- (as a result, we have to recreate bunch of tables due to foreign key constraints)
ALTER TABLE rewarding_epoch RENAME TO combined_rewarding_epoch_v1;
ALTER TABLE epoch_block_signing RENAME TO epoch_block_signing_v1;
ALTER TABLE block_signing_reward RENAME TO block_signing_reward_v1;


CREATE TABLE block_signing_rewarding_epoch
(
    id          INTEGER NOT NULL PRIMARY KEY,
    start_time  TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    end_time    TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    
    -- rewarding budget allocated to this block signing rewarding epoch
    budget      TEXT NOT NULL,
    
    -- indicates whether block signing rewarding/monitoring module is disabled
    disabled    BOOLEAN NOT NULL
);

CREATE TABLE block_signing_rewarding_details
(
    rewarding_epoch_id                INTEGER NOT NULL REFERENCES block_signing_rewarding_epoch (id),

    -- total voting power at the start of the epoch used for determining relative rewards
    total_voting_power_at_epoch_start INTEGER NOT NULL,

    -- total number of blocks processed during this rewarding epoch
    num_blocks                        INTEGER NOT NULL,

    -- the actual amount spent (decreased by missing blocks, etc.)
    spent                             TEXT NOT NULL,

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

-- each issuance rewarding is happening daily based on credentials/deposits issued on particular day
CREATE TABLE ticketbook_issuance_epoch (
    date                DATE            NOT NULL UNIQUE PRIMARY KEY,
    first_deposit_id    INTEGER,
    last_deposit_id     INTEGER,
    budget              TEXT            NOT NULL,
    spent               TEXT            NOT NULL,
    rewarding_tx        TEXT,
    rewarding_error     TEXT
);

CREATE TABLE ticketbook_issuance_reward (
    date                        DATE    NOT NULL REFERENCES ticketbook_issuance_epoch(date),
    operator_account            TEXT    NOT NULL,
    amount                      TEXT    NOT NULL,
    whitelisted                 BOOLEAN NOT NULL,
    api_endpoint                TEXT    NOT NULL,
    issued_partial_ticketbooks  INTEGER NOT NULL,
    share_of_issued_ticketbooks FLOAT   NOT NULL,

    UNIQUE (date, operator_account)
);
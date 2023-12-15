/*
 * Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

CREATE TABLE rewarding_epoch
(
    id              INTEGER                     NOT NULL PRIMARY KEY,
    start_time      TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    end_time        TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    budget          TEXT                        NOT NULL,
    spent           TEXT                        NOT NULL,
    rewarding_tx    TEXT,
    rewarding_error TEXT
);

CREATE TABLE epoch_block_signing
(
    rewarding_epoch_id                INTEGER NOT NULL PRIMARY KEY REFERENCES rewarding_epoch (id),
    total_voting_power_at_epoch_start INTEGER NOT NULL,
    num_blocks                        INTEGER NOT NULL,
    budget                            TEXT    NOT NULL
);

CREATE TABLE block_signing_reward
(
    rewarding_epoch_id          INTEGER NOT NULL REFERENCES rewarding_epoch (id),
    validator_consensus_address TEXT    NOT NULL,
    operator_account            TEXT    NOT NULL,
    amount                      TEXT    NOT NULL,
    voting_power                BIGINT  NOT NULL,
    voting_power_share          TEXT    NOT NULL,
    signed_blocks               INTEGER NOT NULL,
    signed_blocks_percent       TEXT    NOT NULL,

    UNIQUE (rewarding_epoch_id, operator_account)
);

CREATE TABLE epoch_credential_issuance
(
    rewarding_epoch_id       INTEGER NOT NULL PRIMARY KEY REFERENCES rewarding_epoch (id),
    dkg_epoch_id             INTEGER NOT NULL,--     currently not incrementing, needs to change
    total_issued_credentials INTEGER NOT NULL,
    budget                   TEXT    NOT NULL
);

CREATE TABLE malformed_credential
(
    rewarding_epoch_id INTEGER NOT NULL REFERENCES rewarding_epoch (id)

);

CREATE TABLE credential_issuance_reward
(
    rewarding_epoch_id           INTEGER NOT NULL REFERENCES rewarding_epoch (id),
    operator_account             TEXT    NOT NULL,
    amount                       TEXT    NOT NULL,
    api_endpoint                 TEXT    NOT NULL,
    issued_partial_credentials   INTEGER NOT NULL,
    issued_credentials_share     TEXT    NOT NULL,
    validated_issued_credentials INTEGER NOT NULL,

    UNIQUE (rewarding_epoch_id, operator_account)
);

-- CREATE TABLE credential_verification_reward
-- (
--
-- );


/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */


DROP TABLE blinded_shares;
CREATE TABLE blinded_shares
(
    id                  INTEGER                     NOT NULL PRIMARY KEY,
--    added request_uuid to tie it to deposit and actual share data
    request_uuid        TEXT                        NOT NULL REFERENCES ticketbook_deposit(request_uuid),
    status              TEXT                        NOT NULL,
    device_id           TEXT                        NOT NULL,
    credential_id       TEXT                        NOT NULL,
--    replaced the explicit data field in favour of separate table alongside
--    the information on the number of shares available (need min. threshold)
    available_shares    INTEGER                     NOT NULL DEFAULT 0,
    error_message       TEXT                        DEFAULT NULL,
    created             TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    updated             TIMESTAMP WITHOUT TIME ZONE NOT NULL
);

CREATE UNIQUE INDEX blinded_shares_index ON blinded_shares (credential_id, device_id);


CREATE TABLE ticketbook_deposit (
    deposit_id                      INTEGER PRIMARY KEY NOT NULL,
    deposit_tx_hash                 TEXT NOT NULL,
    requested_on                    TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    request_uuid                    TEXT UNIQUE NOT NULL,
    deposit_amount                  TEXT NOT NULL,
    client_pubkey                   BLOB NOT NULL,
    ed25519_deposit_private_key     BLOB NOT NULL
);

CREATE TABLE partial_blinded_wallet (
    corresponding_deposit   INTEGER NOT NULL REFERENCES ticketbook_deposit(deposit_id),
    epoch_id                INTEGER NOT NULL,
    expiration_date         DATE NOT NULL,
    node_id                 INTEGER NOT NULL,
    created                 TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    blinded_signature       BLOB NOT NULL
);

CREATE TABLE partial_blinded_wallet_failure (
    corresponding_deposit   INTEGER NOT NULL REFERENCES ticketbook_deposit(deposit_id),
    epoch_id                INTEGER NOT NULL,
    expiration_date         DATE NOT NULL,
    node_id                 INTEGER NOT NULL,
    created                 TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    failure_message         TEXT NOT NULL
);

-- copied (+rev) from nym-api
CREATE TABLE master_verification_key (
    epoch_id                INTEGER PRIMARY KEY NOT NULL,
    serialization_revision  INTEGER NOT NULL,
    serialised_key BLOB     NOT NULL
);

CREATE TABLE global_coin_index_signatures (
    -- we can only have a single entry
    epoch_id                INTEGER PRIMARY KEY NOT NULL,
    serialization_revision  INTEGER NOT NULL,

    -- combined signatures for all indices
    serialised_signatures   BLOB NOT NULL
);

CREATE TABLE global_expiration_date_signatures (
    expiration_date         DATE NOT NULL UNIQUE PRIMARY KEY,
    epoch_id                INTEGER NOT NULL,
    serialization_revision  INTEGER NOT NULL,

    -- combined signatures for all tuples issued for given day
    serialised_signatures   BLOB NOT NULL
);

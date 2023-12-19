/*
 * Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */


DROP TABLE signed_deposit;

-- represents information about credentials issued in that epoch so that the other parties could make their appropriate queries
-- note: before this information can be returned to a client (of the API), it needs to be signed first
CREATE TABLE epoch_credentials
(
    epoch_id     INTEGER NOT NULL PRIMARY KEY UNIQUE,
    start_id     INTEGER NOT NULL,
    total_issued INTEGER NOT NULL
);

-- particular credential issued in this epoch
CREATE TABLE issued_credential
(
    id                         INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    epoch_id                   INTEGER NOT NULL,
    tx_hash                    VARCHAR NOT NULL UNIQUE,
    bs58_partial_credential    VARCHAR NOT NULL,
    bs58_signature             VARCHAR NOT NULL,
    joined_private_commitments VARCHAR NOT NULL,
    joined_public_attributes   VARCHAR NOT NULL
);
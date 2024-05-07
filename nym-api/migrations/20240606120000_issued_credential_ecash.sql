/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */


DROP TABLE issued_credential;

-- particular ecash credential issued in this epoch
CREATE TABLE issued_credential
(
    id                         INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    epoch_id                   INTEGER NOT NULL,
    deposit_id                 INTEGER NOT NULL UNIQUE,
    bs58_partial_credential    VARCHAR NOT NULL,
    bs58_signature             VARCHAR NOT NULL,
    joined_private_commitments VARCHAR NOT NULL,
    expiration_date            DATETIME NOT NULL
);
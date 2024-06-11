/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

CREATE TABLE coin_indices_signatures
(
    epoch_id                INTEGER NOT NULL PRIMARY KEY,
    signatures              TEXT NOT NULL
);

CREATE TABLE ecash_credentials
(
    id                     INTEGER                                                                  NOT NULL PRIMARY KEY AUTOINCREMENT,

--     introduce a way for us to introduce breaking changes in serialization
    serialization_revision INTEGER                                                                  NOT NULL,

    credential_data        BLOB                                                                     NOT NULL UNIQUE,
    epoch_id               INTEGER                                                                  NOT NULL,
    expired                BOOLEAN                                                                  NOT NULL,
    consumed               BOOLEAN                                                                  NOT NULL
);
/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

DROP TABLE coconut_credentials;
CREATE TABLE coconut_credentials
(
    id                     INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,

--     introduce a way for us to introduce breaking changes in serialization
    serialization_revision INTEGER NOT NULL,
    credential_type        TEXT    NOT NULL,
    credential_data        BLOB    NOT NULL,
    epoch_id               INTEGER NOT NULL,
    consumed               BOOLEAN NOT NULL,
    expired                BOOLEAN NOT NULL
);
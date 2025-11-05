/*
 * Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

CREATE TABLE emergency_credential
(
    id         INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,

    type       TEXT    NOT NULL,

    -- don't define any strict schema on the content as it might be implementation-dependant
    content    BLOB    NOT NULL,

    expiration TIMESTAMP WITHOUT TIME ZONE
);

-- no point in allowing duplicate data
CREATE UNIQUE INDEX emergency_credential_unique_type_content
    ON emergency_credential (type, content);

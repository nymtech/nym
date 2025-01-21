/*
 * Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */


ALTER TABLE message_store
    RENAME TO message_store_old;

-- add new column with message timestamp. 
-- note: we can't simply alter existing table to add it since the default value is non-constant
CREATE TABLE message_store
(
    id                  INTEGER                     NOT NULL PRIMARY KEY AUTOINCREMENT,
    client_address_bs58 TEXT                        NOT NULL,
    content             BLOB                        NOT NULL,
    timestamp           TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO message_store(id, client_address_bs58, content)
SELECT id, client_address_bs58, content
FROM message_store_old;
DROP TABLE message_store_old;


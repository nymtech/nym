/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

CREATE TABLE issued_freepass
(
    id            INTEGER PRIMARY KEY CHECK (id = 0),
    current_nonce INTEGER NOT NULL
);

INSERT INTO issued_freepass(id, current_nonce) VALUES (0,0);
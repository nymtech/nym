/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

CREATE TABLE ISSUED_FREEPASS
(
    id            INTEGER PRIMARY KEY CHECK (id = 0),
    current_nonce INTEGER NOT NULL
);
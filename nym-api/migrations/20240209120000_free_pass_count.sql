/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

CREATE TABLE issued_freepass
(
    id     INTEGER PRIMARY KEY CHECK (id = 0),
    issued INTEGER NOT NULL
);

INSERT INTO issued_freepass(id, issued)
VALUES (0, 0);
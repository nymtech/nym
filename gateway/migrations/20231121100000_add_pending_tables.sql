/*
 * Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

CREATE TABLE pending
(
    id                  INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    credential           TEXT NOT NULL,
    address              TEXT NOT NULL,
    api_url              TEXT NOT NULL
);
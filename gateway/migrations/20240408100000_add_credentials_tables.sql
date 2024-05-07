/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */


CREATE TABLE credentials
(
    id                  INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    credentials         TEXT NOT NULL
);

CREATE TABLE pending
(
    id                  INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    credential          TEXT NOT NULL,
    gateway_address     TEXT NOT NULL,
    api_urls            TEXT NOT NULL,
    proposal_id         INTEGER
);
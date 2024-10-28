/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

CREATE TABLE sessions_active
(
    client_address  TEXT                        NOT NULL PRIMARY KEY UNIQUE,
    start_time      TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    typ             TEXT                        NOT NULL
);

CREATE TABLE sessions_finished
(
    id          INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    day         DATE    NOT NULL,
    duration_ms INTEGER NOT NULL,
    typ         TEXT    NOT NULL
);

CREATE TABLE sessions_unique_users
(
    day             DATE    NOT NULL,
    client_address  TEXT    NOT NULL,
    PRIMARY KEY (day, client_address)
);
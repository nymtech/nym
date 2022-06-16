/*
 * Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

CREATE TABLE service_statistics
(
    id                         INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    requested_service          VARCHAR NOT NULL,
    request_processed_bytes    INTEGER NOT NULL,
    response_processed_bytes   INTEGER NOT NULL,
    interval_seconds           INTEGER NOT NULL,
    timestamp                  DATETIME NOT NULL
);

CREATE TABLE gateway_statistics
(
    id                         INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    inbox_count                INTEGER NOT NULL,
    timestamp                  DATETIME NOT NULL
);
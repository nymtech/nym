/*
 * Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

CREATE TABLE METADATA
(
    id                    INTEGER PRIMARY KEY CHECK (id = 0),
    last_processed_height INTEGER NOT NULL
);
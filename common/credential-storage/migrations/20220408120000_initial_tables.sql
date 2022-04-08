/*
 * Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

CREATE TABLE coconut_credentials
(
    id                  INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    credential          TEXT    NOT NULL UNIQUE
);
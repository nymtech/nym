/*
 * Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

create table spent_credentials
(
    id                  INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    credential          TEXT NOT NULL,
    serial_number       TEXT NOT NULL,
    gateway_address     TEXT NOT NULL,
    proposal_id         TEXT NOT NULL

);

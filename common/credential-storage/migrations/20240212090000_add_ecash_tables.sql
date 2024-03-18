/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

CREATE TABLE coin_indices_signatures
(
    epoch_id                TEXT NOT NULL PRIMARY KEY,
    signatures              TEXT NOT NULL
);
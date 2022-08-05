/*
 * Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

ALTER TABLE mixnode_details
    RENAME TO mixnode_details_old;

CREATE TABLE mixnode_details
(
    id       INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    mix_id   INTEGER NOT NULL UNIQUE,
    owner    VARCHAR NOT NULL,
    identity_key VARCHAR NOT NULL
);

CREATE INDEX `mixnode_mix_id_index` ON `mixnode_details` (`id`, `mix_id`);

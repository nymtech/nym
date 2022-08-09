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

-- we might have to "empty" more tables here

ALTER TABLE rewarding_report
    RENAME TO rewarding_report_old;

ALTER TABLE interval_rewarding
    RENAME TO interval_rewarding_deprecated;

CREATE TABLE rewarding_report
(
    absolute_epoch_id        INTEGER NOT NULL,

    eligible_mixnodes            INTEGER NOT NULL,

    possibly_unrewarded_mixnodes INTEGER NOT NULL,

    FOREIGN KEY (absolute_epoch_id) REFERENCES interval_rewarding_deprecated (id)
);

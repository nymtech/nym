/*
 * Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

CREATE TABLE IF NOT EXISTS node_families (
    family_id           BIGINT  PRIMARY KEY,
    name                TEXT    NOT NULL,
    description         TEXT    NOT NULL,
    owner               TEXT    NOT NULL,
    family_stake_unym   BIGINT,
    members_count       INTEGER NOT NULL,
    created_at          BIGINT  NOT NULL,
    last_updated_utc    BIGINT  NOT NULL
);

CREATE TABLE IF NOT EXISTS node_family_members (
    node_id     BIGINT PRIMARY KEY,
    family_id   BIGINT NOT NULL REFERENCES node_families (family_id) ON DELETE CASCADE,
    joined_at   BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_node_family_members_family_id
    ON node_family_members (family_id);

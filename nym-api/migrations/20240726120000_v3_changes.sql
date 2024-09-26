/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */
 
-- we don't have to be keeping track of the gateway owner; it will make things easier in code
ALTER TABLE gateway_details DROP COLUMN owner;
ALTER TABLE mixnode_details DROP COLUMN owner;

-- NOTE: this column is made `NOT NULL UNIQUE` in code during `migrate_v3_database` call!
ALTER TABLE gateway_details ADD node_id INTEGER;

-- a hacky table-flag to indicate whether the v3 migration has been run
CREATE TABLE v3_migration_info (
    id INTEGER PRIMARY KEY CHECK (id = 0)
);

--CREATE TABLE node_historical_performance (
--    contract_node_id    INTEGER NOT NULL,
--    date                DATE NOT NULL,
--    performance         FLOAT NOT NULL,
--
--    UNIQUE(contract_node_id, date);
--)

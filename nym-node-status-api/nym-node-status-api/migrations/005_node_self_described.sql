ALTER TABLE nym_nodes ADD COLUMN self_described TEXT;
ALTER TABLE nym_nodes ADD COLUMN bond_info TEXT;

-- # Why recreate tables?
-- I need DELETE with CASCADE functionality, but ALTER TABLE doesn't support
-- adding constraints (which CASCADE is). So I recreate tables with proper
-- constraints and fill them with existing data.

-- To avoid invalidating existing FK references, temporarily disable FK enforcement.
PRAGMA foreign_keys=off;

DROP INDEX IF EXISTS idx_nym_nodes_packet_stats_raw_node_id_timestamp_utc;

ALTER TABLE nym_node_descriptions RENAME TO _nym_node_descriptions_old;
ALTER TABLE nym_nodes_packet_stats_raw RENAME TO _nym_nodes_packet_stats_raw_old;
ALTER TABLE nym_node_daily_mixing_stats RENAME TO _nym_node_daily_mixing_stats_old;

CREATE TABLE nym_node_descriptions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    node_id INTEGER UNIQUE NOT NULL,
    moniker VARCHAR,
    website VARCHAR,
    security_contact VARCHAR,
    details VARCHAR,
    last_updated_utc INTEGER NOT NULL,
    FOREIGN KEY (node_id) REFERENCES nym_nodes (node_id) ON DELETE CASCADE
);

CREATE TABLE nym_nodes_packet_stats_raw (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    node_id INTEGER NOT NULL,
    timestamp_utc INTEGER NOT NULL,
    packets_received INTEGER,
    packets_sent INTEGER,
    packets_dropped INTEGER,
    FOREIGN KEY (node_id) REFERENCES nym_nodes (node_id) ON DELETE CASCADE
);

CREATE INDEX idx_nym_nodes_packet_stats_raw_node_id_timestamp_utc ON nym_nodes_packet_stats_raw (node_id, timestamp_utc);

CREATE TABLE nym_node_daily_mixing_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    node_id INTEGER NOT NULL,
    total_stake BIGINT NOT NULL,
    date_utc VARCHAR NOT NULL,
    packets_received INTEGER DEFAULT 0,
    packets_sent INTEGER DEFAULT 0,
    packets_dropped INTEGER DEFAULT 0,
    FOREIGN KEY (node_id) REFERENCES nym_nodes (node_id) ON DELETE CASCADE,
    UNIQUE (node_id, date_utc) -- This constraint automatically creates an index
);

INSERT INTO nym_node_descriptions SELECT * FROM _nym_node_descriptions_old;
INSERT INTO nym_nodes_packet_stats_raw SELECT * FROM _nym_nodes_packet_stats_raw_old;
INSERT INTO nym_node_daily_mixing_stats SELECT * FROM _nym_node_daily_mixing_stats_old;

DROP TABLE _nym_node_descriptions_old;
DROP TABLE _nym_nodes_packet_stats_raw_old;
DROP TABLE _nym_node_daily_mixing_stats_old;

PRAGMA foreign_keys=on;

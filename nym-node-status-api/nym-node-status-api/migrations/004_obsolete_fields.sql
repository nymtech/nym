ALTER TABLE mixnodes DROP COLUMN blacklisted;
ALTER TABLE gateways DROP COLUMN blacklisted;

CREATE TABLE nym_nodes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    node_id INTEGER NOT NULL UNIQUE,
    ed25519_identity_pubkey VARCHAR NOT NULL UNIQUE,
    total_stake INTEGER NOT NULL,
    ip_addresses TEXT NOT NULL,
    mix_port INTEGER NOT NULL,
    x25519_sphinx_pubkey VARCHAR NOT NULL UNIQUE,
    node_role TEXT NOT NULL,
    supported_roles TEXT NOT NULL,
    performance VARCHAR NOT NULL,
    entry TEXT,
    last_updated_utc INTEGER NOT NULL
);

CREATE INDEX idx_nym_nodes_node_id ON nym_nodes (node_id);
CREATE INDEX idx_nym_nodes_ed25519_identity_pubkey ON nym_nodes (ed25519_identity_pubkey);

CREATE TABLE nym_node_descriptions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    node_id INTEGER UNIQUE NOT NULL,
    moniker VARCHAR,
    website VARCHAR,
    security_contact VARCHAR,
    details VARCHAR,
    last_updated_utc INTEGER NOT NULL,
    FOREIGN KEY (node_id) REFERENCES nym_nodes (node_id)
);

CREATE TABLE nym_nodes_packet_stats_raw (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    node_id INTEGER NOT NULL,
    timestamp_utc INTEGER NOT NULL,
    packets_received INTEGER,
    packets_sent INTEGER,
    packets_dropped INTEGER,
    FOREIGN KEY (node_id) REFERENCES nym_nodes (node_id)
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
    FOREIGN KEY (node_id) REFERENCES nym_nodes (node_id),
    UNIQUE (node_id, date_utc) -- This constraint automatically creates an index
);

-- Removing UNIQUE constraint on nym_nodes
-- https://www.sqlite.org/lang_altertable.html

-- To avoid invalidating existing FK references, temporarily disable FK enforcement.
PRAGMA foreign_keys=off;

CREATE TABLE nym_nodes_new (
    node_id INTEGER PRIMARY KEY,
    ed25519_identity_pubkey VARCHAR NOT NULL,
    total_stake INTEGER NOT NULL,
    ip_addresses TEXT NOT NULL, -- JSON serialized
    mix_port INTEGER NOT NULL,
    x25519_sphinx_pubkey VARCHAR NOT NULL,
    node_role TEXT NOT NULL, -- JSON serialized
    supported_roles TEXT NOT NULL, -- JSON serialized
    performance VARCHAR NOT NULL,
    entry TEXT, -- JSON serialized
    self_described TEXT, -- JSON serialized
    bond_info TEXT, -- JSON serialized
    last_updated_utc INTEGER NOT NULL
);

-- columns are misaligned because old nym_nodes has 2 subsequently added columns
-- which come at the end of schema definition.
-- To correctly insert values into corresponding columns, named columns are required
INSERT INTO nym_nodes_new (
    node_id,
    ed25519_identity_pubkey,
    total_stake,
    ip_addresses,
    mix_port,
    x25519_sphinx_pubkey,
    node_role,
    supported_roles,
    performance,
    entry,
    self_described,
    bond_info,
    last_updated_utc
)
SELECT
    existing.node_id,
    existing.ed25519_identity_pubkey,
    existing.total_stake,
    existing.ip_addresses,
    existing.mix_port,
    existing.x25519_sphinx_pubkey,
    existing.node_role,
    existing.supported_roles,
    existing.performance,
    existing.entry,
    existing.self_described,
    existing.bond_info,
    existing.last_updated_utc
FROM nym_nodes as existing;

DROP INDEX IF EXISTS idx_nym_nodes_node_id;
DROP INDEX IF EXISTS idx_nym_nodes_ed25519_identity_pubkey;

DROP TABLE nym_nodes;

ALTER TABLE nym_nodes_new RENAME TO nym_nodes;

CREATE INDEX idx_nym_nodes_node_id ON nym_nodes (node_id);
CREATE INDEX idx_nym_nodes_ed25519_identity_pubkey ON nym_nodes (ed25519_identity_pubkey);


PRAGMA foreign_keys=on;

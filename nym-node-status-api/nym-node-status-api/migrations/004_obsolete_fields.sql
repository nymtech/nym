ALTER TABLE mixnodes DROP COLUMN blacklisted;
ALTER TABLE gateways DROP COLUMN blacklisted;

CREATE TABLE nym_nodes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    node_id INTEGER NOT NULL UNIQUE,
    ed25519_identity_pubkey VARCHAR NOT NULL UNIQUE,
    ip_addresses VARCHAR,
    mix_port INTEGER NOT NULL,
    x25519_sphinx_pubkey VARCHAR NOT NULL UNIQUE,
    node_role VARCHAR NOT NULL,
    supported_roles VARCHAR NOT NULL,
    performance VARCHAR NOT NULL,
    entry VARCHAR,
    last_updated_utc INTEGER NOT NULL
);

CREATE INDEX idx_nym_nodes_mix_id ON mixnodes (mix_id);
CREATE INDEX idx_nym_nodes_identity_key ON mixnodes (identity_key);

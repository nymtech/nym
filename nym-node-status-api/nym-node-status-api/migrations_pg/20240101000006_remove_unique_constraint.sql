-- Removing UNIQUE constraints on nym_nodes
-- In PostgreSQL, we can drop constraints directly without recreating the table

-- Drop the unique constraints
ALTER TABLE nym_nodes DROP CONSTRAINT IF EXISTS nym_nodes_ed25519_identity_pubkey_key;
ALTER TABLE nym_nodes DROP CONSTRAINT IF EXISTS nym_nodes_x25519_sphinx_pubkey_key;

-- The columns and indexes remain, only the unique constraints are removed
-- The existing indexes idx_nym_nodes_node_id and idx_nym_nodes_ed25519_identity_pubkey remain unchanged
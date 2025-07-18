-- Add partial indexes for NOT NULL filtering to improve performance of /explorer/v3/nodes endpoint
-- PostgreSQL version

-- Index for queries filtering on self_described IS NOT NULL
CREATE INDEX IF NOT EXISTS idx_nym_nodes_self_described_not_null
ON nym_nodes(node_id)
WHERE self_described IS NOT NULL;

-- Index for queries filtering on bond_info IS NOT NULL
CREATE INDEX IF NOT EXISTS idx_nym_nodes_bond_info_not_null
ON nym_nodes(node_id)
WHERE bond_info IS NOT NULL;

-- Composite index for queries filtering on both bond_info AND self_described
CREATE INDEX IF NOT EXISTS idx_nym_nodes_bond_self_described
ON nym_nodes(node_id)
WHERE bond_info IS NOT NULL AND self_described IS NOT NULL;

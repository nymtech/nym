-- Convert remaining TEXT columns that store JSON to JSONB
ALTER TABLE nym_nodes 
    ALTER COLUMN node_role TYPE JSONB USING node_role::JSONB,
    ALTER COLUMN supported_roles TYPE JSONB USING supported_roles::JSONB,
    ALTER COLUMN entry TYPE JSONB USING entry::JSONB,
    ALTER COLUMN self_described TYPE JSONB USING self_described::JSONB,
    ALTER COLUMN bond_info TYPE JSONB USING bond_info::JSONB;
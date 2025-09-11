ALTER TABLE nym_nodes ADD COLUMN self_described TEXT;
ALTER TABLE nym_nodes ADD COLUMN bond_info TEXT;

-- PostgreSQL doesn't need table recreation for adding ON DELETE CASCADE
-- We can drop and recreate the foreign key constraints directly

-- Drop existing foreign key constraints
ALTER TABLE nym_node_descriptions DROP CONSTRAINT IF EXISTS nym_node_descriptions_node_id_fkey;
ALTER TABLE nym_nodes_packet_stats_raw DROP CONSTRAINT IF EXISTS nym_nodes_packet_stats_raw_node_id_fkey;
ALTER TABLE nym_node_daily_mixing_stats DROP CONSTRAINT IF EXISTS nym_node_daily_mixing_stats_node_id_fkey;

-- Add foreign key constraints with ON DELETE CASCADE
ALTER TABLE nym_node_descriptions 
    ADD CONSTRAINT nym_node_descriptions_node_id_fkey 
    FOREIGN KEY (node_id) REFERENCES nym_nodes (node_id) ON DELETE CASCADE;

ALTER TABLE nym_nodes_packet_stats_raw 
    ADD CONSTRAINT nym_nodes_packet_stats_raw_node_id_fkey 
    FOREIGN KEY (node_id) REFERENCES nym_nodes (node_id) ON DELETE CASCADE;

ALTER TABLE nym_node_daily_mixing_stats 
    ADD CONSTRAINT nym_node_daily_mixing_stats_node_id_fkey 
    FOREIGN KEY (node_id) REFERENCES nym_nodes (node_id) ON DELETE CASCADE;
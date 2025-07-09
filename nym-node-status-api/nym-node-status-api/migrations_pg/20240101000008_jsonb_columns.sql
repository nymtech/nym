-- Convert ip_addresses column from TEXT to JSONB for better type safety
ALTER TABLE nym_nodes 
    ALTER COLUMN ip_addresses TYPE JSONB USING ip_addresses::JSONB;
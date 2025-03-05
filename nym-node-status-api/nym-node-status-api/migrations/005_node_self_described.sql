ALTER TABLE nym_nodes ADD COLUMN self_described TEXT;
ALTER TABLE nym_nodes ADD COLUMN bond_info TEXT;
ALTER TABLE nym_nodes ADD COLUMN active INTEGER CHECK (active in (0, 1)) NOT NULL DEFAULT 0;

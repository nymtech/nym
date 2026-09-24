-- Config-score inputs captured from a node's self-description, consumed by the config-score
-- materialiser (see the `network-monitor-config-score` capability). Every column is nullable with
-- no default, matching the other describe-derived columns: NULL means "not retrieved" and must stay
-- distinct from a retrieved value, so a failed or absent query never reads as a known answer. A node
-- that has not been described scores config-unavailable rather than blocking anything.

-- Self-reported binary version (raw semver string, parsed at score time). NULL until described.
ALTER TABLE nym_node ADD COLUMN reported_version TEXT;

-- Self-reported binary name; the config score gates on this being 'nym-node'. NULL until described.
ALTER TABLE nym_node ADD COLUMN binary_name TEXT;

-- Whether the operator accepted the terms and conditions, as self-reported. NULL until described, so
-- a node we could not query is not assumed to have refused them.
ALTER TABLE nym_node ADD COLUMN accepted_terms_and_conditions BOOLEAN;

-- The node's self-reported on-chain address, used to look up its balance and feegrant. NULL when the
-- node reports none (e.g. one predating the v2 auxiliary endpoint).
ALTER TABLE nym_node ADD COLUMN declared_chain_address TEXT;

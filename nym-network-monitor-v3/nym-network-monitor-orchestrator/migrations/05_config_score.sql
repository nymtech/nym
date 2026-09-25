-- Schema for the config-score capability: the per (node, epoch) config score, the inputs it is
-- computed from, and the per-node on-chain cache it draws on. See the `network-monitor-config-score`
-- capability.

-- 1. Config-score inputs captured from a node's self-description, alongside a `bonded` flag the node
--    refresher maintains. The describe-derived columns are nullable with no default, matching the
--    other describe columns: NULL means "not retrieved" and must stay distinct from a retrieved
--    value, so a failed or absent query never reads as a known answer.

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

-- Whether the node is still bonded on chain, maintained by the node refresher: every node in the
-- contract's current bond set is marked bonded, and any node it no longer sees is marked unbonded.
-- Since a node_id is never reused, an unbonded node is definitively gone (a rebond takes a fresh
-- node_id), so config-score materialisation and the chain-capability sweep filter on this to avoid
-- scoring or endlessly querying nodes that will never be reachable again. Defaults to true so
-- existing rows read as bonded until the first refresh reconciles them.
ALTER TABLE nym_node ADD COLUMN bonded BOOLEAN NOT NULL DEFAULT TRUE;

-- 2. Per-node cache of on-chain standing (balance + feegrant), used to derive the chain-interaction
--    part of the config score without querying the chain on the materialisation path. A rebuildable
--    cache: losing it means config scores briefly read as unable-to-transact until the refresh sweep
--    repopulates it.
CREATE TABLE node_chain_capability
(
    -- The node these capabilities belong to.
    node_id             INTEGER PRIMARY KEY         NOT NULL REFERENCES nym_node (node_id),

    -- The node's on-chain balance as a serialised Coin (amount + denom), in the Coin's Display form
    -- and parsed back with FromStr. The full Coin is kept rather than a bare amount so the value
    -- stays auditable if denoms ever vary; the RAW balance (not a sufficiency flag) is kept so the
    -- minimum-balance threshold can change and take effect at score time without re-querying.
    balance             TEXT                        NOT NULL,

    -- Whether the node's on-chain address holds at least one feegrant allowance. Inherently boolean,
    -- so unlike the balance there is nothing to defer to score time.
    is_feegrant_grantee BOOLEAN                     NOT NULL,

    -- When these capabilities were last successfully queried (observability).
    refreshed_at        TIMESTAMP WITHOUT TIME ZONE NOT NULL,

    -- When this row is next due to be re-queried: refreshed_at + the configured TTL + a random
    -- jitter. Jittering the due time per node keeps a population cached together from all falling due
    -- at the same instant, so the sweep re-queries a spread rather than the whole fleet at once.
    next_refresh_due_at TIMESTAMP WITHOUT TIME ZONE NOT NULL
);

-- 3. Per (mixnet_epoch, node) config score: the third performance input beside the liveness and
--    stress aggregates, but NOT a probe test_kind and NOT the score-and-count shape - it carries its
--    own decomposition. One row per bonded node per epoch, computed at the epoch transition from the
--    node's self-description and cached on-chain standing.
CREATE TABLE mixnet_epoch_config_score
(
    mixnet_epoch                  INTEGER                                      NOT NULL,

    node_id                       INTEGER                                      NOT NULL REFERENCES nym_node (node_id),

    -- The config score in [0, 1].
    score                         REAL CHECK ( score >= 0.0 AND score <= 1.0 ) NOT NULL,

    -- Weighted versions behind the on-chain head. NULL when there was no self-description or the
    -- reported version did not parse, in which case the score is a hard zero.
    versions_behind               INTEGER,

    -- The subcomponents that produced the score, kept so a zero can be attributed: a stale version,
    -- unaccepted terms, the wrong binary, no self-description, or an inability to transact on chain.
    accepted_terms_and_conditions BOOLEAN                                      NOT NULL,
    runs_nym_node_binary          BOOLEAN                                      NOT NULL,
    self_described_available      BOOLEAN                                      NOT NULL,
    has_sufficient_tokens         BOOLEAN                                      NOT NULL,
    is_feegrant_grantee           BOOLEAN                                      NOT NULL,

    PRIMARY KEY (mixnet_epoch, node_id)
);

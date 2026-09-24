-- Per-node cache of on-chain standing (balance + feegrant), used to derive the chain-interaction
-- part of the config score without querying the chain on the materialisation path. A rebuildable
-- cache like the aggregates: losing it means config scores briefly read as unable-to-transact until
-- the refresh sweep repopulates it. See the `network-monitor-config-score` capability.
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

    -- When these capabilities were last successfully queried, so the refresh sweep can tell a fresh
    -- row from a stale one.
    refreshed_at        TIMESTAMP WITHOUT TIME ZONE NOT NULL
);

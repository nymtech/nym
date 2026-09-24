-- Per (mixnet_epoch, node) config score: the third performance input beside the liveness and stress
-- aggregates, but NOT a probe test_kind and NOT the score-and-count shape - it carries its own
-- decomposition (see the network-monitor-config-score capability). One row per bonded node per epoch,
-- computed at the epoch transition from the node's self-description and cached on-chain standing.
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

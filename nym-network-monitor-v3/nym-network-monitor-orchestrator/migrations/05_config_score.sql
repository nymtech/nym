/*
 * Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

-- ---------------------------------------------------------------------------
-- nym_node_description: the config-score inputs
-- ---------------------------------------------------------------------------

-- Read from the same complete self-description as every other column and required like them, so a
-- node that cannot report them is not described at all.
--
-- The table is emptied first because SQLite refuses to add a NOT NULL column without a default to a
-- table holding rows. Nothing references it, and the startup refresh rebuilds it.
DELETE FROM nym_node_description;

-- Self-reported binary version, as a raw semver string parsed at score time.
ALTER TABLE nym_node_description ADD COLUMN reported_version TEXT NOT NULL;

-- Self-reported binary name. The config score requires it to be 'nym-node'.
ALTER TABLE nym_node_description ADD COLUMN binary_name TEXT NOT NULL;

-- Whether the operator accepted the terms and conditions.
ALTER TABLE nym_node_description ADD COLUMN accepted_terms_and_conditions BOOLEAN NOT NULL;

-- The node's self-reported on-chain address, bech32-encoded, by which its balance and feegrant are
-- looked up. Validated before it is stored.
ALTER TABLE nym_node_description ADD COLUMN declared_chain_address TEXT NOT NULL;

-- ---------------------------------------------------------------------------
-- node_chain_capability: what a node's on-chain address can do
-- ---------------------------------------------------------------------------

-- A cache of each node's on-chain standing, kept warm by its own sweep so that scoring never queries
-- the chain. Losing it costs nothing permanent: a node whose standing is not cached is left unscored
-- until the sweep reaches it.
CREATE TABLE node_chain_capability
(
    node_id             INTEGER PRIMARY KEY REFERENCES nym_node_bond (node_id) NOT NULL,

    -- The address's balance, as a Coin in its Display form. Kept raw rather than as a sufficiency
    -- flag, so a changed minimum takes effect at score time without re-querying.
    balance             TEXT                                                   NOT NULL,

    -- Whether the address holds at least one feegrant allowance.
    is_feegrant_grantee BOOLEAN                                                NOT NULL,

    -- When this standing was last queried.
    refreshed_at        TIMESTAMP                                              NOT NULL,

    -- When it is next due to be queried: refreshed_at plus the TTL plus a random jitter, so that a
    -- population cached together does not all fall due at the same instant.
    next_refresh_due_at TIMESTAMP                                              NOT NULL
);

-- ---------------------------------------------------------------------------
-- mixnet_epoch_config_score: how a node was configured as an epoch began
-- ---------------------------------------------------------------------------

-- A snapshot of each described node's configuration, taken when the epoch begins and filed under it.
-- Never backfilled: there is no history to replay, so a past epoch would only ever get today's state.
-- Kept apart from the aggregates because it is not a mean over runs and carries its own
-- decomposition. An absent row is an absent value: the node was not described, or its on-chain
-- standing was not cached yet.
CREATE TABLE mixnet_epoch_config_score
(
    -- Absolute id of the mixnet epoch this value is filed under.
    mixnet_epoch                  INTEGER                                      NOT NULL,

    -- When that epoch began. What eviction ages a row by.
    epoch_start                   TIMESTAMP                                    NOT NULL,

    node_id                       INTEGER REFERENCES nym_node_bond (node_id)   NOT NULL,

    score                         REAL CHECK ( score >= 0.0 AND score <= 1.0 ) NOT NULL,

    -- Weighted versions behind the newest version on chain. NULL when the reported version did not
    -- parse, which forces the score to zero.
    versions_behind               INTEGER,

    -- What produced the score, so that a low one can be attributed: a stale version, unaccepted
    -- terms, the wrong binary, or an inability to transact on chain.
    accepted_terms_and_conditions BOOLEAN                                      NOT NULL,
    runs_nym_node_binary          BOOLEAN                                      NOT NULL,
    has_sufficient_tokens         BOOLEAN                                      NOT NULL,
    is_feegrant_grantee           BOOLEAN                                      NOT NULL,

    PRIMARY KEY (mixnet_epoch, node_id)
);

-- Supports the eviction sweep.
CREATE INDEX idx_mixnet_epoch_config_score_epoch_start ON mixnet_epoch_config_score (epoch_start);

/*
 * Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

-- ---------------------------------------------------------------------------
-- mixnet_epoch_aggregate: what a node was worth over one epoch, per kind
-- ---------------------------------------------------------------------------

-- Computed once, when an epoch begins, from the runs each kind's results table holds for the window
-- preceding it, and served from here thereafter. Materialised rather than computed on read so a value
-- handed to a consumer cannot move underneath it.
--
-- One table for every kind rather than one per kind: unlike the results, an aggregate has the same
-- shape whatever its kind measured. A row exists only where something was measured, so an absent
-- row is an absent value and can never be mistaken for a measured zero.
CREATE TABLE mixnet_epoch_aggregate
(
    -- Absolute id of the mixnet epoch this value is filed under, as the mixnet contract counts them.
    -- Named in full because this schema already uses `epoch` for the unrelated sphinx
    -- `key_rotation_id`.
    mixnet_epoch INTEGER                                         NOT NULL,

    -- When that epoch began, which is also where the window this value covers ends. What eviction
    -- ages a row by.
    epoch_start  TIMESTAMP                                       NOT NULL,

    node_id      INTEGER REFERENCES nym_node_bond (node_id)      NOT NULL,

    test_kind    TEXT REFERENCES test_kind (name)                NOT NULL,

    -- The mean of the scores of the runs in the window.
    score        REAL CHECK ( score >= 0.0 AND score <= 1.0 )    NOT NULL,

    -- How many runs that mean was taken over: the only ground truth about how much evidence stands
    -- behind a value.
    samples      INTEGER CHECK ( samples > 0 )                   NOT NULL,

    PRIMARY KEY (mixnet_epoch, node_id, test_kind)
);

-- Supports the eviction sweep.
CREATE INDEX idx_mixnet_epoch_aggregate_epoch_start ON mixnet_epoch_aggregate (epoch_start);

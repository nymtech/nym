/*
 * Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

-- ---------------------------------------------------------------------------
-- nym_node_description: whether a gateway enforces zk-nyms
-- ---------------------------------------------------------------------------

-- Read from the gateway's root details in the same complete reading as every other column. A route
-- session claims free bandwidth, so a run draws only gateways that say they do not enforce.
--
-- NULL for a node that does not report the entry-gateway role, and on a description written before
-- this column existed until the next complete reading replaces it. An unread flag must not read as
-- "does not enforce", which is why the descriptions are kept with a NULL rather than emptied.
ALTER TABLE nym_node_description
    ADD COLUMN enforces_zk_nyms BOOLEAN CHECK (gateway_enabled OR enforces_zk_nyms IS NULL);

-- ---------------------------------------------------------------------------
-- five_hop_mixnet_run: one row per route run handed out
-- ---------------------------------------------------------------------------

-- A route run is not a test kind: it targets no node and takes no in-flight row. This table is both
-- its cadence and its in-flight state. A run is in flight until it is submitted or its expiry
-- passes, and the hand-out keeps it to one at a time by checking inside its write transaction. A run
-- that expires unsubmitted is left as it is until eviction.
CREATE TABLE five_hop_mixnet_run
(
    -- The id the run is handed out and submitted under.
    id           INTEGER   NOT NULL PRIMARY KEY AUTOINCREMENT,

    -- When the run was handed out. The next one is due an interval after the start of the last
    -- submitted run.
    started_at   TIMESTAMP NOT NULL,

    -- When the run stops being in flight unless submitted: `started_at` plus the run timeout,
    -- materialised as the in-flight leases are.
    expires_at   TIMESTAMP NOT NULL,

    -- When its sessions were accepted. NULL while in flight, and for good once it expired unsubmitted.
    submitted_at TIMESTAMP
);

-- Supports the start of the last submitted run, and the eviction sweep.
CREATE INDEX idx_five_hop_mixnet_run_started_at ON five_hop_mixnet_run (started_at);

-- ---------------------------------------------------------------------------
-- five_hop_mixnet_run_gateway: the gateways a run was given
-- ---------------------------------------------------------------------------

-- Written with the run, so that a submission naming any other gateway can be refused. Node ids only:
-- the identity key handed out beside each comes from the bond.
CREATE TABLE five_hop_mixnet_run_gateway
(
    run_id  INTEGER NOT NULL REFERENCES five_hop_mixnet_run (id),
    node_id INTEGER NOT NULL REFERENCES nym_node_bond (node_id),

    PRIMARY KEY (run_id, node_id)
);

-- ---------------------------------------------------------------------------
-- five_hop_mixnet_session: one row per gateway of a submitted run
-- ---------------------------------------------------------------------------

-- A route session either has its batch, stored exactly as received and never expanded into a row per
-- packet, or failed and has its error instead. The epoch and the record count are read from the
-- batch when it is stored, so a failed session has neither.
CREATE TABLE five_hop_mixnet_session
(
    run_id          INTEGER NOT NULL,
    gateway_node_id INTEGER NOT NULL,

    -- Absolute id of the mixnet epoch the batch names, that of the active set its run read. The
    -- session belongs to this epoch, whatever the clock said. NULL exactly when the session failed.
    mixnet_epoch    INTEGER,

    -- Number of records in the batch. NULL exactly when the session failed.
    record_count    INTEGER,

    -- Why the session produced no batch. NULL exactly when it has one.
    error           TEXT,

    -- The session's records in the order sent, behind the header naming the gateway and the epoch.
    batch           BLOB,

    -- Whether the discard rule kept the session for scoring, judged against its run when submitted.
    -- NULL exactly when the session failed.
    kept            BOOLEAN,

    PRIMARY KEY (run_id, gateway_node_id),

    -- a session exists only for a gateway its run was given
    FOREIGN KEY (run_id, gateway_node_id) REFERENCES five_hop_mixnet_run_gateway (run_id, node_id),

    CHECK ((error IS NULL) = (batch IS NOT NULL)),
    CHECK ((mixnet_epoch IS NULL) = (batch IS NULL)),
    CHECK ((record_count IS NULL) = (batch IS NULL)),
    CHECK ((kept IS NULL) = (batch IS NULL))
);

-- Supports reading the sessions that name an epoch.
CREATE INDEX idx_five_hop_mixnet_session_mixnet_epoch ON five_hop_mixnet_session (mixnet_epoch);

-- ---------------------------------------------------------------------------
-- mixnet_epoch_five_hop_liveness: the five-hop liveness of a mixnode in one epoch
-- ---------------------------------------------------------------------------

-- Computed once, as an epoch begins, and never recomputed. Kept apart from the aggregates because it
-- is not a mean over runs. A row is either measured, from the sessions naming the epoch that just
-- ended, for a mixnode the scoring strategy could score, or carried forward, for a mixnode with a
-- measured value in recent epochs but none in this one. A mixnode with neither has no row, never a
-- zero. Only the strategy's result is kept: whatever it counted on the way is its own.
CREATE TABLE mixnet_epoch_five_hop_liveness
(
    -- Absolute id of the mixnet epoch this value is filed under, the one after the epoch measured.
    mixnet_epoch INTEGER                                      NOT NULL,

    -- When that epoch began. What eviction ages a row by.
    epoch_start  TIMESTAMP                                    NOT NULL,

    -- Unlike every other per-node table, not a reference to the bond: a batch names mixnodes from
    -- the active set nym-api reported, which can include one bonded since the last refresh. Its
    -- value is kept, and applies once a refresh records the bond.
    node_id      INTEGER                                      NOT NULL,

    -- The node's five-hop liveness in this epoch: as the scoring strategy gave it when measured, else
    -- its newest measured value decayed by the epochs since.
    score        REAL CHECK ( score >= 0.0 AND score <= 1.0 ) NOT NULL,

    -- Whether the score was measured in this epoch rather than carried forward. Only a measured score
    -- is carried forward.
    measured     BOOLEAN                                      NOT NULL,

    PRIMARY KEY (mixnet_epoch, node_id)
);

-- Supports the eviction sweep.
CREATE INDEX idx_mixnet_epoch_five_hop_liveness_epoch_start ON mixnet_epoch_five_hop_liveness (epoch_start);

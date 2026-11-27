/*
 * Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

-- ---------------------------------------------------------------------------
-- nym_node_description: whether a gateway enforces zk-nyms
-- ---------------------------------------------------------------------------

-- Read from the gateway's root details in the same complete reading as every other column. A
-- five-hop mixnet session claims free bandwidth, so a run draws only gateways that say they do not
-- enforce.
--
-- NULL for a node that does not report the entry-gateway role, and on a description written before
-- this column existed until the next complete reading replaces it. An unread flag must not read as
-- "does not enforce", which is why the descriptions are kept with a NULL rather than emptied.
ALTER TABLE nym_node_description
    ADD COLUMN enforces_zk_nyms BOOLEAN CHECK (gateway_enabled OR enforces_zk_nyms IS NULL);

-- ---------------------------------------------------------------------------
-- five_hop_mixnet_run: one row per five-hop mixnet run handed out
-- ---------------------------------------------------------------------------

-- A five-hop mixnet run is not a test kind: it targets no node and takes no in-flight row. This
-- table is both its cadence and its in-flight state. A run is in flight until it is submitted or
-- its expiry passes, and the hand-out keeps it to one at a time by checking inside its write
-- transaction. A run that expires unsubmitted is left as it is until eviction.
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
-- five_hop_mixnet_session: one row per session of a submitted run that produced a batch
-- ---------------------------------------------------------------------------

-- A five-hop mixnet session's batch, stored exactly as received and never expanded into a row per
-- packet. A session that failed has no row: it is logged as its run is submitted.
CREATE TABLE five_hop_mixnet_session
(
    run_id          INTEGER NOT NULL REFERENCES five_hop_mixnet_run (id),

    -- The gateway the session ran through, as its batch names it. Not a reference to the bond: the
    -- gateways a run was given are not stored.
    gateway_node_id INTEGER NOT NULL,

    -- Absolute id of the mixnet epoch the batch names, that of the active set its run read. The
    -- session belongs to this epoch, whatever the clock said.
    mixnet_epoch    INTEGER NOT NULL,

    -- The session's records in the order sent, behind the header naming the gateway and the epoch.
    batch           BLOB    NOT NULL,

    -- Whether the discard rule kept the session for scoring, judged against its run when submitted.
    kept            BOOLEAN NOT NULL,

    PRIMARY KEY (run_id, gateway_node_id)
);

-- Supports reading the sessions that name an epoch.
CREATE INDEX idx_five_hop_mixnet_session_mixnet_epoch ON five_hop_mixnet_session (mixnet_epoch);

-- ---------------------------------------------------------------------------
-- mixnet_epoch_five_hop_liveness: the five-hop liveness of a mixnode in one epoch
-- ---------------------------------------------------------------------------

-- Computed once, as an epoch begins, and never recomputed. Kept apart from the aggregates because it
-- is not a mean over runs. A row is either measured, from the sessions naming the epoch that just
-- ended, for a mixnode the scoring strategy could score, or carried forward, for a mixnode with a
-- row under the epoch before but none measured in this one. A mixnode with neither has no row,
-- never a zero. Only the strategy's result is kept: whatever it counted on the way is its own.
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
    -- the score filed under the epoch before, faded by one epoch towards 1. A carried score is no
    -- longer filed once it is within half a percent of 1.
    score        REAL CHECK ( score >= 0.0 AND score <= 1.0 ) NOT NULL,

    -- Whether the score was measured in this epoch rather than carried forward.
    measured     BOOLEAN                                      NOT NULL,

    PRIMARY KEY (mixnet_epoch, node_id)
);

-- Supports the eviction sweep.
CREATE INDEX idx_mixnet_epoch_five_hop_liveness_epoch_start ON mixnet_epoch_five_hop_liveness (epoch_start);

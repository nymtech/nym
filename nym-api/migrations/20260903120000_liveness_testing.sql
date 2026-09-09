/*
 * Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

-- Liveness results from the network monitor orchestrators, stored separately from
-- `nym_node_stress_testing_result` rather than sharing it behind a kind discriminator.
--
-- The two kinds are separate streams end to end: each has its own submission endpoint, its own
-- per-signer replay high-water mark, and its own watermark on the orchestrator side. They are also
-- aggregated independently and weighted independently downstream. A shared table would add a
-- discriminator that every read has to remember to filter on, and forgetting it silently blends
-- two populations - one of which is deliberately shipped at weight zero - into one score.
--
-- Unlike the stress table, liveness rows may describe a GATEWAY as well as a mixnode. Nothing here
-- records which: the submitted score is a single average whose shape is identical for both roles,
-- and the per-interface breakdown that would reveal the role stays on the orchestrator, under the
-- run's own row, where its operator read surface serves it.
CREATE TABLE nym_node_liveness_result
(
    -- Stable, store-assigned handle for this measurement. AUTOINCREMENT rather than a bare rowid
    -- alias so ids are never reused if a retention sweep is ever added: reuse would recreate, one
    -- layer up, the id-collision bug that the `submitter_pubkey` key below exists to avoid.
    id               INTEGER                     NOT NULL PRIMARY KEY AUTOINCREMENT,

    -- Orchestrator-local testrun id that produced this result. Retained so a row can be traced
    -- back to the submitting orchestrator's own read surface, but deliberately NOT part of any
    -- key: it is an autoincrement counter in a database that is not treated as durable state, so
    -- wiping it restarts the counter at 1 and every resubmitted id would collide with a row
    -- already stored here, to be dropped silently by `INSERT OR IGNORE` behind a 200 response
    -- until the counter climbed back past its previous high-water mark. Restoring that database
    -- from a backup is worse: an id is reused for a DIFFERENT measurement, so the stored row and
    -- the orchestrator's row disagree about what it means. This is not hypothetical - it is the
    -- defect that migration 20260806120000 exists to fix on the stress table, and there is no
    -- reason to reintroduce it on a brand new one.
    testrun_id       INTEGER                     NOT NULL,

    -- Base58-encoded ed25519 identity key of the submitting orchestrator.
    submitter_pubkey TEXT                        NOT NULL,

    -- as on the stress table, legacy separate mixnode/gateway tables mean there is no single
    -- table to point a reference constraint at
    node_id          INTEGER                     NOT NULL,

    -- Delivery ratio in [0.0, 1.0], averaged over the fixed set of interfaces the run's probe was
    -- expected to exercise, with an interface that produced no measurement counted as zero. The
    -- averaging happens on the orchestrator because the denominator comes from the probed role,
    -- which never reaches the wire.
    result           REAL                        NOT NULL,

    -- Distinguishes a genuine zero (the node answered and dropped everything) from the node being
    -- unreachable, which a bare 0.0 in `result` cannot express.
    was_reachable    BOOLEAN                     NOT NULL,

    test_timestamp   TIMESTAMP WITHOUT TIME ZONE NOT NULL,

    -- Identity is the measurement itself, which a wipe of the orchestrator's database cannot
    -- repeat, and which makes the orchestrator's at-least-once resends idempotent: a resent row
    -- carries the identical triple, being read back from the same source row.
    --
    -- Column order is deliberate: the constraint holds on the set regardless, but leading with
    -- `(node_id, test_timestamp)` lets this index also serve the per-node windowed average, which
    -- is the only read query against this table.
    UNIQUE (node_id, test_timestamp, submitter_pubkey)
);

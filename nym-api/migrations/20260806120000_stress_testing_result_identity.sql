/*
 * Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

-- Rebuild `nym_node_stress_testing_result` so that a measurement's identity no longer depends on
-- an orchestrator-local row counter.
--
-- The original primary key was `(testrun_id, submitter_pubkey)`. `testrun.id` is an AUTOINCREMENT
-- counter in the orchestrator's own SQLite database, which is not treated as durable state: if it
-- is wiped the counter restarts from 1 while the orchestrator keeps its (durable, on-chain)
-- ed25519 identity. Every resubmitted id then collided with a row already stored here and was
-- discarded by `INSERT OR IGNORE`, silently, behind a 200 response, until the counter climbed back
-- past its previous high-water mark - which takes about as long as the wiped database had been
-- alive. Restoring that database from a backup was worse: an id was reused for a *different*
-- measurement, so the stored row and the orchestrator's row disagreed about what the id meant.
--
-- Identity is therefore now the measurement itself, `(node_id, test_timestamp, submitter_pubkey)`,
-- which a wipe cannot repeat. Dedupe of the orchestrator's at-least-once retries is unaffected: a
-- resent row carries the identical triple, since it is read back from the same source row.
CREATE TABLE nym_node_stress_testing_result_new
(
    -- Stable, store-assigned handle for this measurement, used by the read endpoints.
    -- AUTOINCREMENT (rather than a bare rowid alias) so that ids are never reused once a
    -- retention sweep starts deleting rows - reuse would recreate, one layer up, exactly the
    -- id-collision bug this migration exists to fix.
    id               INTEGER                     NOT NULL PRIMARY KEY AUTOINCREMENT,

    -- Orchestrator-local testrun id that produced this result. Retained so a row can be traced
    -- back to the submitting orchestrator's own read surface, but no longer part of any key.
    testrun_id       INTEGER                     NOT NULL,

    -- Base58-encoded ed25519 identity key of the submitting orchestrator.
    submitter_pubkey TEXT                        NOT NULL,

    -- unfortunately, due to legacy reasons we have separate tables for mixnodes and gateways
    -- so that we can't put a reference constraint here
    node_id          INTEGER                     NOT NULL,

    result           REAL                        NOT NULL,

    was_reachable    BOOLEAN                     NOT NULL,

    test_timestamp   TIMESTAMP WITHOUT TIME ZONE NOT NULL,

    -- Column order is deliberate: the constraint is on the set, so it holds regardless, but
    -- leading with `(node_id, test_timestamp)` lets this index also serve the per-node windowed
    -- average that is the only read query against this table. The previous primary key led with
    -- `testrun_id` and could not serve it at all.
    UNIQUE (node_id, test_timestamp, submitter_pubkey)
);

-- `INSERT OR IGNORE` rather than a plain insert: the old primary key permitted two distinct
-- testrun ids from one submitter to carry the same (node, timestamp) pair. That should not exist
-- in practice - a node can hold only one in-progress run at a time and timestamps are
-- sub-microsecond - but if it does, dropping the duplicate is correct and, more importantly, keeps
-- the migration from aborting and wedging nym-api startup.
-- Ordering by test_timestamp assigns ids oldest-measurement-first.
INSERT OR IGNORE INTO nym_node_stress_testing_result_new (testrun_id, submitter_pubkey, node_id,
                                                          result, was_reachable, test_timestamp)
SELECT testrun_id, submitter_pubkey, node_id, result, was_reachable, test_timestamp
FROM nym_node_stress_testing_result
ORDER BY test_timestamp;

DROP TABLE nym_node_stress_testing_result;

ALTER TABLE nym_node_stress_testing_result_new RENAME TO nym_node_stress_testing_result;

/*
 * Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

CREATE TABLE nym_node_stress_testing_result
(
    -- Orchestrator-local testrun id that produced this result. Paired with `submitter_pubkey`
    -- it uniquely identifies a measurement and lets us dedupe retried submissions (the
    -- orchestrator uses at-least-once delivery and may re-POST the same row after a crash
    -- between a successful POST and its watermark update).
    testrun_id       INTEGER                     NOT NULL,

    -- Base58-encoded ed25519 identity key of the submitting orchestrator. Part of the primary
    -- key so distinct orchestrators can coincidentally share a `testrun_id` without colliding.
    submitter_pubkey TEXT                        NOT NULL,

    -- unfortunately, due to legacy reasons we have separate tables for mixnodes and gateways
    -- so that we can't put a reference constraint here
    node_id          INTEGER                     NOT NULL,

    result           REAL                        NOT NULL,

    was_reachable    BOOLEAN                     NOT NULL,

    test_timestamp   TIMESTAMP WITHOUT TIME ZONE NOT NULL,

    PRIMARY KEY (testrun_id, submitter_pubkey)
);

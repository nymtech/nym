/*
 * Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

CREATE TABLE nym_node_stress_testing_result
(
    -- unfortunately, due to legacy reasons we have separate tables for mixnodes and gateways
    -- so that we can't put a reference constraint here
    node_id        INTEGER                     NOT NULL,

    result         REAL                        NOT NULL,

    was_reachable  BOOLEAN                     NOT NULL,

    test_timestamp TIMESTAMP WITHOUT TIME ZONE NOT NULL
);
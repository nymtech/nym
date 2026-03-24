/*
 * Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

CREATE TABLE testrun
(
    -- Surrogate primary key.
    id                         INTEGER                                            NOT NULL PRIMARY KEY AUTOINCREMENT,

    -- Discriminator for the type of node under test; future-proofs the table for when we start testing gateways.
    test_type                  TEXT CHECK ( test_type IN ('mixnode', 'gateway') ) NOT NULL,

    -- When this testrun has been performed.
    test_timestamp             TIMESTAMP WITHOUT TIME ZONE                        NOT NULL,

    -- Duration of the Noise handshake on the ingress (responder) side, in microseconds.
    -- NULL if the handshake did not complete.
    ingress_noise_handshake_us INTEGER,

    -- Duration of the Noise handshake on the egress (initiator) side, in microseconds.
    -- NULL if the handshake did not complete.
    egress_noise_handshake_us  INTEGER,

    -- Number of sphinx packets sent to the node under test.
    packets_sent               INTEGER                                            NOT NULL DEFAULT 0,

    -- Number of sphinx packets received back from the node under test.
    packets_received           INTEGER                                            NOT NULL DEFAULT 0,

    -- RTT of the initial probe packet in microseconds, approximating baseline latency.
    -- NULL if the probe did not complete successfully.
    approximate_latency_us     INTEGER,

    -- RTT distribution (in microseconds) computed over all received packets.
    -- All four columns are NULL together when no packets were received.
    packets_rtt_min_us         INTEGER,
    packets_rtt_mean_us        INTEGER,
    packets_rtt_max_us         INTEGER,
    packets_rtt_std_dev_us     INTEGER,

    -- Batch send latency distribution (in microseconds) recorded during the load test.
    -- All four columns are NULL together when no batches were sent.
    sending_latency_min_us     INTEGER,
    sending_latency_mean_us    INTEGER,
    sending_latency_max_us     INTEGER,
    sending_latency_std_dev_us INTEGER,

    -- Whether any packet was received with a duplicate ID during this test run.
    received_duplicates        BOOLEAN                                            NOT NULL,

    -- Human-readable description of the first error that caused the test to abort.
    -- NULL if the test completed without error.
    error                      TEXT

);

CREATE TABLE nym_node
(
    -- Node ID as assigned by the mixnet contract.
    node_id               INTEGER PRIMARY KEY         NOT NULL,

    -- Ed25519 identity key of the node, base58-encoded.
    -- A node_id always maps to exactly one identity_key and is never reassigned.
    -- The inverse is not true: the same identity_key may appear under multiple node_ids
    -- if the operator unbonds and rebonds, receiving a new contract-assigned node_id.
    identity_key          TEXT                        NOT NULL,

    -- When this node was last observed as bonded in the contract.
    last_seen_bonded      TIMESTAMP WITHOUT TIME ZONE NOT NULL,

    -- Mixnet socket address (host:port) at which the node accepts sphinx packets.
    mixnet_socket_address TEXT,

    -- X25519 public key used for Noise handshakes, base58-encoded.
    -- NULL if retrieval from the node failed.
    noise_key             TEXT,

    -- Sphinx public key used for packet encryption, base58-encoded.
    -- NULL if retrieval from the node failed.
    -- Always NULL/non-NULL together with key_rotation_id.
    sphinx_key            TEXT,

    -- Key rotation epoch ID that the sphinx_key belongs to.
    -- NULL if retrieval from the node failed.
    -- Always NULL/non-NULL together with sphinx_key.
    key_rotation_id       INTEGER,

    -- The most recent test run performed against this node. NULL if never tested.
    -- Set to NULL automatically when the referenced testrun row is evicted.
    last_testrun          INTEGER                     REFERENCES testrun (id) ON DELETE SET NULL,

    CHECK ((sphinx_key IS NULL) = (key_rotation_id IS NULL))
);

-- Tracks nodes that currently have a test run in progress.
-- At most one row per node (enforced by the PRIMARY KEY on node_id).
-- A row is inserted when a run is dispatched and deleted when it completes or is abandoned.
CREATE TABLE testrun_in_progress
(
    -- The node currently being tested.
    node_id    INTEGER PRIMARY KEY REFERENCES nym_node (node_id) NOT NULL,

    -- When the in-progress run was started; used to detect stale/hung runs.
    started_at TIMESTAMP WITHOUT TIME ZONE                       NOT NULL
)
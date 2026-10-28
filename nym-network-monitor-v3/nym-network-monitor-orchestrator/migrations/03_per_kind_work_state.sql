/*
 * Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

-- Reshapes the schema around test kinds. A kind is one probe against one role (mixnode_liveness,
-- gateway_liveness, mixnode_stress), and each gets its own results table, its own work state keyed
-- by (node, kind) and its own submission watermark. The node registry splits into what the mixnet
-- contract says about a node (its bond) and what the node's own endpoint reported (its
-- description), so a node is either completely described or not described at all.
--
-- Only the bonds are carried across; everything else is recreated EMPTY. Completed results are a
-- retry buffer already submitted every `result_submission_interval`, in-flight leases are orphaned
-- by the restart this migration implies, and the refresh that runs at startup rebuilds the
-- descriptions. The old rows could not become descriptions in any case: they lack the client
-- websocket port that a gateway-capable description requires.
--
-- Restarting testrun ids from 1 is safe: nym-api identifies a stored result by
-- (node_id, test_timestamp, submitter_pubkey) and carries `testrun_id` for traceability only (see
-- its `20260806120000_stress_testing_result_identity` migration).
--
-- Statement order is load-bearing. The bonds are copied out of `nym_node` before it is dropped, and
-- the old tables are dropped children first: with foreign keys enforced, DROP TABLE performs an
-- implicit DELETE, which fails while rows of another table still reference the one being dropped.

-- ---------------------------------------------------------------------------
-- nym_node_bond: what the mixnet contract says about a node
-- ---------------------------------------------------------------------------

-- Written for every bonded node on every refresh, whether or not the node itself answered. Never
-- deleted, so a node that has unbonded stays listed with its last-seen time.
CREATE TABLE nym_node_bond
(
    -- Node ID as assigned by the mixnet contract.
    node_id          INTEGER PRIMARY KEY NOT NULL,

    -- Ed25519 identity key of the node, base58-encoded.
    -- A node_id always maps to exactly one identity_key and is never reassigned.
    -- The inverse is not true: the same identity_key may appear under multiple node_ids
    -- if the operator unbonds and rebonds, receiving a new contract-assigned node_id.
    identity_key     TEXT                NOT NULL,

    -- When this node was last observed as bonded in the contract.
    last_seen_bonded TIMESTAMP           NOT NULL
);

INSERT INTO nym_node_bond (node_id, identity_key, last_seen_bonded)
SELECT node_id, identity_key, last_seen_bonded
FROM nym_node;

-- ---------------------------------------------------------------------------
-- Discard the old tables
-- ---------------------------------------------------------------------------

DROP TABLE testrun_in_progress;

DROP TABLE testrun;

DROP TABLE metadata;

DROP TABLE nym_node;

-- ---------------------------------------------------------------------------
-- nym_node_description: what the node's own endpoint reported
-- ---------------------------------------------------------------------------

-- The node's latest COMPLETE self-description, replaced whole by every successful reading and left
-- untouched by a failed one. A missing row means the node has never been completely described or is
-- no longer bonded, and either way it is ineligible for every kind.
CREATE TABLE nym_node_description
(
    node_id         INTEGER PRIMARY KEY REFERENCES nym_node_bond (node_id) NOT NULL,

    -- Port of the node's mixnet listener. A port rather than a socket address, because the address
    -- under test comes from the per-kind rotation over announced_ips.
    mix_port        INTEGER                                                NOT NULL,

    -- Every ip address the node announced, comma-separated. Canonicalised, deduplicated and sorted on
    -- write, which is what keeps the per-kind rotation over it stable across refreshes. Never empty:
    -- a node announcing no address cannot be described.
    announced_ips   TEXT                                                   NOT NULL,

    -- X25519 public key used for Noise handshakes, base58-encoded.
    noise_key       TEXT                                                   NOT NULL,

    -- Sphinx public key used for packet encryption, base58-encoded, and the key rotation epoch it
    -- belongs to.
    sphinx_key      TEXT                                                   NOT NULL,
    key_rotation_id INTEGER                                                NOT NULL,

    -- The roles the node reports, exactly as it reports them. `gateway_enabled` is the entry-gateway
    -- role; whether the node also exits is not a distinction any test makes. A node reporting
    -- neither is ineligible for every kind.
    mixnode_enabled BOOLEAN                                                NOT NULL,
    gateway_enabled BOOLEAN                                                NOT NULL,

    -- Port of the node's PLAIN client websocket listener, which a gateway liveness probe opens its
    -- client session on. Present exactly when the node reports the gateway role.
    clients_ws_port INTEGER,

    CHECK ((clients_ws_port IS NOT NULL) = gateway_enabled)
);

-- ---------------------------------------------------------------------------
-- test_kind: every kind, listed once
-- ---------------------------------------------------------------------------

-- Every kind column references this table rather than repeating a CHECK list, so adding a kind is
-- one INSERT in a later migration instead of a rebuild of every table naming one, SQLite being
-- unable to alter a CHECK. Keyed by the name itself so that rows stay readable.
CREATE TABLE test_kind
(
    name TEXT PRIMARY KEY NOT NULL
);

INSERT INTO test_kind (name)
VALUES ('mixnode_liveness'),
       ('gateway_liveness'),
       ('mixnode_stress');

-- ---------------------------------------------------------------------------
-- node_test_state: per (node, kind) work state
-- ---------------------------------------------------------------------------

-- Each kind keeps its own staleness position and address rotation, so a 15-minute liveness cadence
-- and a 2-hour stress cadence never fight over one pointer, and a dual-role node is due separately
-- for each of the two liveness kinds.
CREATE TABLE node_test_state
(
    node_id        INTEGER REFERENCES nym_node_bond (node_id) NOT NULL,

    test_kind      TEXT REFERENCES test_kind (name)           NOT NULL,

    -- When this kind last completed a run against the node, which is what the staleness gate reads.
    -- Stored directly rather than derived from the kind's results so that evicting an old result
    -- does not make the node read as never-tested and jump the assignment queue.
    -- NULL while the node has only ever been assigned, never measured.
    last_tested_at TIMESTAMP,

    -- The address handed out for this kind's most recent assignment, used purely as the rotation
    -- pointer into the description's announced_ips. Advances when the assignment is handed out
    -- rather than when a result arrives, so a run that is abandoned still moves the node onto its
    -- next address. NULL until this kind has assigned the node at least once.
    last_tested_ip TEXT,

    -- A row is created by whichever path touches it first: the assignment (which writes only the
    -- rotation pointer) or the result submission (which writes only the timestamp). Hence every
    -- column beyond the key is nullable.
    PRIMARY KEY (node_id, test_kind)
);

-- ---------------------------------------------------------------------------
-- testrun_in_progress: the in-flight dispatch lock set
-- ---------------------------------------------------------------------------

-- Keyed by node_id ALONE, across kinds: a node being stress-tested at high rate while a liveness
-- probe measures it would bias both results, so only one test of any kind may be in flight against
-- a node at a time.
CREATE TABLE testrun_in_progress
(
    -- The node currently being tested.
    node_id    INTEGER PRIMARY KEY REFERENCES nym_node_bond (node_id) NOT NULL,

    -- When the in-progress run was dispatched.
    started_at TIMESTAMP                                              NOT NULL,

    -- When the lease expires and the row becomes reapable, materialised as `started_at` plus the
    -- dispatching kind's lease budget, so the eviction sweep stays a single `expires_at < ?`
    -- comparison and never has to learn about kinds.
    expires_at TIMESTAMP                                              NOT NULL,

    -- What the run was dispatched to measure. This is the AUTHORITATIVE source of the kind, and so of
    -- the results table, when the result comes back: the submission reports only the node and the
    -- address, so without it the orchestrator would depend on the agent echoing back a value the
    -- orchestrator itself chose.
    test_kind  TEXT REFERENCES test_kind (name)                       NOT NULL
);

-- ---------------------------------------------------------------------------
-- submission_watermark: one row per kind
-- ---------------------------------------------------------------------------

-- One per kind, since each kind's results live in their own table under their own ids.
CREATE TABLE submission_watermark
(
    test_kind                 TEXT PRIMARY KEY REFERENCES test_kind (name) NOT NULL,

    -- Id of the newest run in this kind's results table whose batch submission has been
    -- acknowledged. The row is created by the first successful submission, so a missing row (rather
    -- than a NULL column) means "nothing submitted yet, send everything currently stored".
    last_submitted_testrun_id INTEGER                                      NOT NULL
);

-- ---------------------------------------------------------------------------
-- Per-kind results
-- ---------------------------------------------------------------------------

-- One table per kind, each holding the run-level facts plus one column group per interface the kind
-- exercises. Every group carries the same columns, prefixed by the interface it describes, and both
-- the run-level columns and the group are documented once, on mixnode_liveness_testrun. A result
-- carries exactly the interfaces of its kind, so every group of a row is always written.

CREATE TABLE mixnode_liveness_testrun
(
    -- Surrogate primary key, unique only within this kind.
    id                                        INTEGER   NOT NULL PRIMARY KEY AUTOINCREMENT,

    -- The node under test.
    node_id                                   INTEGER   NOT NULL REFERENCES nym_node_bond (node_id),

    -- The address of the node that was actually tested. A node may announce several addresses and
    -- only some of them may be healthy, so the result is meaningless without it.
    tested_address                            TEXT      NOT NULL,

    -- When this testrun has been performed.
    test_timestamp                            TIMESTAMP NOT NULL,

    -- How long the test took to complete, in microseconds, from the point of view of an agent.
    time_taken_us                             INTEGER   NOT NULL,

    -- Human-readable description of the first error that caused the test to abort.
    -- NULL if the test completed without error.
    error                                     TEXT,

    -- mix_forwarding: the node relaying the probe's packets back to the agent.

    -- Duration of the Noise handshake on the ingress (responder) side, in microseconds.
    -- NULL if the handshake did not complete.
    mix_forwarding_ingress_noise_handshake_us INTEGER,

    -- Duration of the Noise handshake on the egress (initiator) side, in microseconds.
    -- NULL if the handshake did not complete.
    mix_forwarding_egress_noise_handshake_us  INTEGER,

    -- The (constant) per-hop delay applied to sphinx packets during the test run, in microseconds.
    mix_forwarding_sphinx_packet_delay_us     INTEGER   NOT NULL,

    -- Number of sphinx packets sent to the node under test.
    mix_forwarding_packets_sent               INTEGER   NOT NULL,

    -- Number of sphinx packets received back from the node under test.
    mix_forwarding_packets_received           INTEGER   NOT NULL,

    -- RTT of the initial probe packet in microseconds, approximating baseline latency.
    -- NULL if the probe did not complete successfully.
    mix_forwarding_approximate_latency_us     INTEGER,

    -- RTT distribution (in microseconds) computed over all received packets.
    -- All five columns are NULL together when no packets were received.
    mix_forwarding_packets_rtt_min_us         INTEGER,
    mix_forwarding_packets_rtt_mean_us        INTEGER,
    mix_forwarding_packets_rtt_median_us      INTEGER,
    mix_forwarding_packets_rtt_max_us         INTEGER,
    mix_forwarding_packets_rtt_std_dev_us     INTEGER,

    -- Whether any packet was received with a duplicate ID against this interface.
    mix_forwarding_received_duplicates        BOOLEAN   NOT NULL
);

-- Supports "all runs for node X, newest first".
CREATE INDEX idx_mixnode_liveness_testrun_node_id_timestamp ON mixnode_liveness_testrun (node_id, test_timestamp DESC);

-- Supports "all runs, newest first", which the composite index above cannot serve, and the eviction
-- sweep.
CREATE INDEX idx_mixnode_liveness_testrun_test_timestamp ON mixnode_liveness_testrun (test_timestamp DESC);

-- Both phases of a gateway run share one client session, so the run-level timing and error cover the
-- two of them together.
CREATE TABLE gateway_liveness_testrun
(
    id                                         INTEGER   NOT NULL PRIMARY KEY AUTOINCREMENT,
    node_id                                    INTEGER   NOT NULL REFERENCES nym_node_bond (node_id),
    tested_address                             TEXT      NOT NULL,
    test_timestamp                             TIMESTAMP NOT NULL,
    time_taken_us                              INTEGER   NOT NULL,
    error                                      TEXT,

    -- client_ingest: the gateway forwarding a client's packets into the mixnet.
    client_ingest_ingress_noise_handshake_us   INTEGER,
    client_ingest_egress_noise_handshake_us    INTEGER,
    client_ingest_sphinx_packet_delay_us       INTEGER   NOT NULL,
    client_ingest_packets_sent                 INTEGER   NOT NULL,
    client_ingest_packets_received             INTEGER   NOT NULL,
    client_ingest_approximate_latency_us       INTEGER,
    client_ingest_packets_rtt_min_us           INTEGER,
    client_ingest_packets_rtt_mean_us          INTEGER,
    client_ingest_packets_rtt_median_us        INTEGER,
    client_ingest_packets_rtt_max_us           INTEGER,
    client_ingest_packets_rtt_std_dev_us       INTEGER,
    client_ingest_received_duplicates          BOOLEAN   NOT NULL,

    -- client_delivery: the gateway delivering mixnet packets to a live client session.
    client_delivery_ingress_noise_handshake_us INTEGER,
    client_delivery_egress_noise_handshake_us  INTEGER,
    client_delivery_sphinx_packet_delay_us     INTEGER   NOT NULL,
    client_delivery_packets_sent               INTEGER   NOT NULL,
    client_delivery_packets_received           INTEGER   NOT NULL,
    client_delivery_approximate_latency_us     INTEGER,
    client_delivery_packets_rtt_min_us         INTEGER,
    client_delivery_packets_rtt_mean_us        INTEGER,
    client_delivery_packets_rtt_median_us      INTEGER,
    client_delivery_packets_rtt_max_us         INTEGER,
    client_delivery_packets_rtt_std_dev_us     INTEGER,
    client_delivery_received_duplicates        BOOLEAN   NOT NULL
);

CREATE INDEX idx_gateway_liveness_testrun_node_id_timestamp ON gateway_liveness_testrun (node_id, test_timestamp DESC);

CREATE INDEX idx_gateway_liveness_testrun_test_timestamp ON gateway_liveness_testrun (test_timestamp DESC);

CREATE TABLE mixnode_stress_testrun
(
    id                                        INTEGER   NOT NULL PRIMARY KEY AUTOINCREMENT,
    node_id                                   INTEGER   NOT NULL REFERENCES nym_node_bond (node_id),
    tested_address                            TEXT      NOT NULL,
    test_timestamp                            TIMESTAMP NOT NULL,
    time_taken_us                             INTEGER   NOT NULL,
    error                                     TEXT,

    -- mix_forwarding
    mix_forwarding_ingress_noise_handshake_us INTEGER,
    mix_forwarding_egress_noise_handshake_us  INTEGER,
    mix_forwarding_sphinx_packet_delay_us     INTEGER   NOT NULL,
    mix_forwarding_packets_sent               INTEGER   NOT NULL,
    mix_forwarding_packets_received           INTEGER   NOT NULL,
    mix_forwarding_approximate_latency_us     INTEGER,
    mix_forwarding_packets_rtt_min_us         INTEGER,
    mix_forwarding_packets_rtt_mean_us        INTEGER,
    mix_forwarding_packets_rtt_median_us      INTEGER,
    mix_forwarding_packets_rtt_max_us         INTEGER,
    mix_forwarding_packets_rtt_std_dev_us     INTEGER,
    mix_forwarding_received_duplicates        BOOLEAN   NOT NULL
);

CREATE INDEX idx_mixnode_stress_testrun_node_id_timestamp ON mixnode_stress_testrun (node_id, test_timestamp DESC);

CREATE INDEX idx_mixnode_stress_testrun_test_timestamp ON mixnode_stress_testrun (test_timestamp DESC);

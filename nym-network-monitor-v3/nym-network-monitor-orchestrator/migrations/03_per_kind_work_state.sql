/*
 * Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

-- Reshapes the work-tracking schema so that every test kind tracks its own progress: staleness and
-- address rotation move off `nym_node` into a table keyed by (node, kind, role), measurements move
-- off `testrun` into a child table keyed by the interface they exercised, and the submission
-- watermark becomes one row per kind.
--
-- The work-tracking tables are RECREATED EMPTY rather than migrated. Nothing here is durable state:
-- completed results are submitted to the nym-api every `result_submission_interval` and only kept
-- locally as a retry buffer, the node registry is rebuilt from the mixnet contract on every refresh,
-- and in-flight rows are leases whose agents are orphaned by the restart this migration implies
-- anyway. Discarding them costs one full-population sweep (every node reads as never-tested) plus
-- whatever had not yet been submitted, and buys a migration with no backfill to get wrong.
--
-- Restarting `testrun.id` from 1 is safe: nym-api identifies a stored result by
-- (node_id, test_timestamp, submitter_pubkey) and carries `testrun_id` for traceability only, so a
-- reused id is no longer mistaken for a resubmission (see its
-- `20260806120000_stress_testing_result_identity` migration).
--
-- Statement order is load-bearing. `nym_node.last_testrun` is a foreign key, and SQLite refuses to
-- DROP COLUMN a column used in one, so the two pointer columns are removed by rebuilding the table.
-- That rebuild has to happen while `testrun` still exists, because renaming `nym_node` rewrites the
-- REFERENCES clauses that point at it, and the tables holding them are dropped immediately after.

-- ---------------------------------------------------------------------------
-- nym_node: drop the per-node test pointers, add the gateway client websocket port
-- ---------------------------------------------------------------------------

ALTER TABLE nym_node
    RENAME TO nym_node_old;

CREATE TABLE nym_node
(
    -- Node ID as assigned by the mixnet contract.
    node_id               INTEGER PRIMARY KEY                                                                  NOT NULL,

    -- Ed25519 identity key of the node, base58-encoded.
    -- A node_id always maps to exactly one identity_key and is never reassigned.
    -- The inverse is not true: the same identity_key may appear under multiple node_ids
    -- if the operator unbonds and rebonds, receiving a new contract-assigned node_id.
    identity_key          TEXT                                                                                 NOT NULL,

    -- When this node was last observed as bonded in the contract.
    last_seen_bonded      TIMESTAMP WITHOUT TIME ZONE                                                          NOT NULL,

    -- Mixnet socket address (host:port) at which the node accepts sphinx packets.
    mixnet_socket_address TEXT,

    -- Every ip address the node announced via its self-described endpoint, comma-separated.
    -- Canonicalised, deduplicated and sorted on write, which is what makes the per-(kind, role)
    -- rotation over it stable across refreshes.
    -- NULL until the node has been successfully queried (same semantics as mixnet_socket_address).
    announced_ips         TEXT,

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

    -- Classification of the node based on the roles reported via its self-described endpoint.
    -- 'unknown' is used both before the node has been successfully queried and when a queried
    -- node reports no roles. Which types a given test kind may assign is decided per kind.
    node_type             TEXT CHECK ( node_type IN ('unknown', 'mixnode', 'gateway', 'mixnode_and_gateway') ) NOT NULL DEFAULT 'unknown',

    -- Port of the node's PLAIN client websocket listener, used to open the client session a
    -- gateway liveness probe runs over. NULL for a node that announces no entry-gateway
    -- interface, and for one that has never been successfully queried.
    clients_ws_port       INTEGER,

    CHECK ((sphinx_key IS NULL) = (key_rotation_id IS NULL))
);

-- The registry itself is worth carrying across: it holds keys learned from nodes that may be
-- transiently unreachable, which the next refresh would otherwise leave NULL.
INSERT INTO nym_node (node_id, identity_key, last_seen_bonded, mixnet_socket_address, announced_ips,
                      noise_key, sphinx_key, key_rotation_id, node_type)
SELECT node_id,
       identity_key,
       last_seen_bonded,
       mixnet_socket_address,
       announced_ips,
       noise_key,
       sphinx_key,
       key_rotation_id,
       node_type
FROM nym_node_old;

-- ---------------------------------------------------------------------------
-- Discard the old work-tracking tables
-- ---------------------------------------------------------------------------

-- Every lease is orphaned by the restart that deploys this migration, so the rows would only keep
-- their nodes out of the assignment queue until the first eviction sweep.
DROP TABLE testrun_in_progress;

DROP TABLE testrun;

-- Superseded by the per-kind watermark table below.
DROP TABLE metadata;

DROP TABLE nym_node_old;

-- ---------------------------------------------------------------------------
-- testrun: run-level facts only
-- ---------------------------------------------------------------------------

CREATE TABLE testrun
(
    -- Surrogate primary key.
    id             INTEGER                                              NOT NULL PRIMARY KEY AUTOINCREMENT,

    -- The node under test.
    node_id        INTEGER                                              NOT NULL REFERENCES nym_node (node_id),

    -- What this run measured, which fixes the set of measurements it is expected to carry and the
    -- submission stream it belongs to.
    test_kind      TEXT CHECK ( test_kind IN ('stress', 'liveness') )   NOT NULL,

    -- Which role of the node this run probed. Distinct from nym_node.node_type, which is the
    -- node's own capability classification and may be both: a dual-role node is probed once per
    -- role, each run recording the role it measured.
    tested_role    TEXT CHECK ( tested_role IN ('mixnode', 'gateway') ) NOT NULL,

    -- The address of the node that was actually tested. A node may announce several addresses and
    -- only some of them may be healthy, so the result is meaningless without it.
    tested_address TEXT                                                 NOT NULL,

    -- When this testrun has been performed.
    test_timestamp TIMESTAMP WITHOUT TIME ZONE                          NOT NULL,

    -- How long the test took to complete, in microseconds, from the point of view of an agent.
    -- Run-level rather than per-measurement: a gateway run holds one session open across both of
    -- its phases, so the two cannot be timed apart.
    time_taken_us  INTEGER                                              NOT NULL,

    -- Human-readable description of the first error that caused the test to abort.
    -- NULL if the test completed without error. Run-level: an aborted run stops the whole test.
    error          TEXT
);

-- Supports efficient "all runs for node X, newest first" lookups.
CREATE INDEX idx_testrun_node_id_timestamp ON testrun (node_id, test_timestamp DESC);

-- Supports efficient "all runs, newest first" lookups (the global testruns pagination endpoint).
-- The composite index above cannot serve this query because its leading column is node_id.
CREATE INDEX idx_testrun_test_timestamp ON testrun (test_timestamp DESC);

-- ---------------------------------------------------------------------------
-- testrun_measurement: one row per interface a run exercised
-- ---------------------------------------------------------------------------

-- A mixnode probe (of either kind) exercises one interface; a gateway liveness probe exercises two,
-- kept separate so that a healthy ingest with a dead delivery is distinguishable from a uniformly
-- half-lossy node. The score reported downstream is the average over the kind's fixed set, so a
-- phase that produced nothing is still recorded, as a zeroed row.
CREATE TABLE testrun_measurement
(
    testrun_id                 INTEGER                                                                            NOT NULL REFERENCES testrun (id) ON DELETE CASCADE,

    -- Which of the node's packet-handling interfaces these counts describe. Names the node FUNCTION
    -- exercised rather than a route, because every value traverses the mixnet in some form. The test
    -- kind deliberately does not appear here: it is a property of the run and already sits on the
    -- parent row.
    interface                  TEXT CHECK ( interface IN ('mix_forwarding', 'client_ingest', 'client_delivery') ) NOT NULL,

    -- Duration of the Noise handshake on the ingress (responder) side, in microseconds.
    -- NULL if the handshake did not complete.
    ingress_noise_handshake_us INTEGER,

    -- Duration of the Noise handshake on the egress (initiator) side, in microseconds.
    -- NULL if the handshake did not complete.
    egress_noise_handshake_us  INTEGER,

    -- The (constant) per-hop delay applied to sphinx packets during the test run, in microseconds.
    sphinx_packet_delay_us     INTEGER                                                                            NOT NULL,

    -- Number of sphinx packets sent to the node under test.
    packets_sent               INTEGER                                                                            NOT NULL DEFAULT 0,

    -- Number of sphinx packets received back from the node under test.
    packets_received           INTEGER                                                                            NOT NULL DEFAULT 0,

    -- RTT of the initial probe packet in microseconds, approximating baseline latency.
    -- NULL if the probe did not complete successfully.
    approximate_latency_us     INTEGER,

    -- RTT distribution (in microseconds) computed over all received packets.
    -- All five columns are NULL together when no packets were received.
    packets_rtt_min_us         INTEGER,
    packets_rtt_mean_us        INTEGER,
    packets_rtt_median_us      INTEGER,
    packets_rtt_max_us         INTEGER,
    packets_rtt_std_dev_us     INTEGER,

    -- Batch send latency distribution (in microseconds) recorded during the load test.
    -- All five columns are NULL together when no batches were sent.
    sending_latency_min_us     INTEGER,
    sending_latency_mean_us    INTEGER,
    sending_latency_median_us  INTEGER,
    sending_latency_max_us     INTEGER,
    sending_latency_std_dev_us INTEGER,

    -- Whether any packet was received with a duplicate ID against this interface.
    received_duplicates        BOOLEAN                                                                            NOT NULL,

    -- A run exercises any given interface at most once, and the pair is also the only lookup key
    -- (reassembling a run's measurements), so it serves as the primary key.
    PRIMARY KEY (testrun_id, interface)
);

-- ---------------------------------------------------------------------------
-- node_test_state: per (node, kind, role) work state
-- ---------------------------------------------------------------------------

-- Replaces nym_node.last_testrun and nym_node.last_tested_ip. Splitting the state per kind is what
-- stops a 15-minute liveness cadence and a 2-hour stress cadence from fighting over one staleness
-- pointer and one rotation cursor. The key carries the ROLE as well, because the two liveness probes
-- are different measurements of the same node: under a (node_id, test_kind) key, a dual-role node's
-- mixnode-liveness run would advance the very timestamp gating its gateway-liveness eligibility, so
-- it would alternate roles across cycles instead of being measured in both.
CREATE TABLE node_test_state
(
    node_id         INTEGER                                              NOT NULL REFERENCES nym_node (node_id),

    test_kind       TEXT CHECK ( test_kind IN ('stress', 'liveness') )   NOT NULL,

    tested_role     TEXT CHECK ( tested_role IN ('mixnode', 'gateway') ) NOT NULL,

    -- When this pairing last completed a run against the node, which is what the staleness gate
    -- reads. Stored directly rather than joined through last_testrun_id so that evicting an old
    -- result does not make the node read as never-tested and jump the assignment queue.
    -- NULL while the node has only ever been assigned, never measured.
    last_tested_at  TIMESTAMP WITHOUT TIME ZONE,

    -- The most recent completed run of this pairing. Set to NULL automatically when that run is
    -- evicted; staleness is unaffected because it lives on last_tested_at.
    last_testrun_id INTEGER                                              REFERENCES testrun (id) ON DELETE SET NULL,

    -- The address handed out for this pairing's most recent assignment, used purely as the rotation
    -- pointer into nym_node.announced_ips. Advances when the assignment is handed out rather than
    -- when a result arrives, so a run that is abandoned still moves the node onto its next address.
    -- NULL until this pairing has assigned the node at least once.
    last_tested_ip  TEXT,

    -- A row is created by whichever path touches the pairing first: the assignment (which writes
    -- only the rotation pointer) or the result submission (which writes only the timestamp and run
    -- id). Hence every column beyond the key is nullable.
    PRIMARY KEY (node_id, test_kind, tested_role)
);

-- ---------------------------------------------------------------------------
-- testrun_in_progress: the in-flight dispatch lock set
-- ---------------------------------------------------------------------------

-- Still keyed by node_id ALONE, across kinds and roles: a node being stress-tested at high rate
-- while a liveness probe measures it would bias both results, so only one test of any kind may be
-- in flight against a node at a time.
CREATE TABLE testrun_in_progress
(
    -- The node currently being tested.
    node_id     INTEGER PRIMARY KEY REFERENCES nym_node (node_id)    NOT NULL,

    -- When the in-progress run was dispatched.
    started_at  TIMESTAMP WITHOUT TIME ZONE                          NOT NULL,

    -- When the lease expires and the row becomes reapable, materialised as `started_at` plus the
    -- dispatching kind's lease budget. Stored rather than derived so the eviction sweep stays a
    -- single `expires_at < ?` comparison and never has to learn about kinds: a future kind that
    -- runs for minutes needs no change to eviction.
    expires_at  TIMESTAMP WITHOUT TIME ZONE                          NOT NULL,

    -- What the run was dispatched to measure.
    test_kind   TEXT CHECK ( test_kind IN ('stress', 'liveness') )   NOT NULL,

    -- Which role of the node the run was dispatched against. This is the AUTHORITATIVE source of
    -- the role when the result comes back: the completed run records the role it measured, while
    -- the submission reports only the node and the address, so without it the orchestrator would
    -- depend on the agent echoing back a value the orchestrator itself chose.
    tested_role TEXT CHECK ( tested_role IN ('mixnode', 'gateway') ) NOT NULL
);

-- ---------------------------------------------------------------------------
-- submission_watermark: one row per submission stream
-- ---------------------------------------------------------------------------

-- Replaces metadata.last_submitted_testrun_id. One shared watermark cannot serve two destinations:
-- the first liveness submission would drag the stress watermark past unsubmitted rows.
CREATE TABLE submission_watermark
(
    test_kind                 TEXT PRIMARY KEY CHECK ( test_kind IN ('stress', 'liveness') ) NOT NULL,

    -- Id of the newest run of this kind whose batch submission has been acknowledged. The row is
    -- created by the first successful submission, so a missing row (rather than a NULL column)
    -- means "nothing submitted yet, send everything currently stored".
    last_submitted_testrun_id INTEGER                                                        NOT NULL
);

/*
 * Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

-- Adds gateway_stress, the gateway liveness test under a stress profile. Every column is
-- documented once, on mixnode_liveness_testrun and gateway_liveness_testrun in
-- 03_per_kind_work_state.sql.
INSERT INTO test_kind (name)
VALUES ('gateway_stress');

CREATE TABLE gateway_stress_testrun
(
    id                                         INTEGER   NOT NULL PRIMARY KEY AUTOINCREMENT,
    node_id                                    INTEGER   NOT NULL REFERENCES nym_node_bond (node_id),
    tested_address                             TEXT      NOT NULL,
    test_timestamp                             TIMESTAMP NOT NULL,
    time_taken_us                              INTEGER   NOT NULL,
    error                                      TEXT,

    -- client_ingest
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

    -- client_delivery
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

CREATE INDEX idx_gateway_stress_testrun_node_id_timestamp ON gateway_stress_testrun (node_id, test_timestamp DESC);

CREATE INDEX idx_gateway_stress_testrun_test_timestamp ON gateway_stress_testrun (test_timestamp DESC);

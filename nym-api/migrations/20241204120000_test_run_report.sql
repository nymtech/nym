/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

CREATE TABLE monitor_run_report
(
    monitor_run_id      INTEGER PRIMARY KEY REFERENCES monitor_run (id),
    network_reliability FLOAT   NOT NULL,
    packets_sent        INTEGER NOT NULL,
    packets_received    INTEGER NOT NULL
);

CREATE TABLE monitor_run_score
(
--     mixnode or gateway
    typ            TEXT    NOT NULL,
    monitor_run_id INTEGER NOT NULL REFERENCES monitor_run_report (monitor_run_id),
    rounded_score  INTEGER NOT NULL,
    nodes_count    INTEGER NOT NULL
);

CREATE INDEX monitor_run_score_id ON monitor_run_score (monitor_run_id);
/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

CREATE INDEX IF NOT EXISTS monitor_run_timestamp on monitor_run(timestamp);
CREATE INDEX IF NOT EXISTS monitor_run_id on monitor_run(id);
CREATE INDEX IF NOT EXISTS testing_route_monitor_run_id on testing_route(monitor_run_id)

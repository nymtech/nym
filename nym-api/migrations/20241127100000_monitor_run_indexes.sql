/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

create index monitor_run_timestamp on monitor_run(timestamp);
create index monitor_run_id on monitor_run(id);
create index testing_route_monitor_run_id on testing_route(monitor_run_id)

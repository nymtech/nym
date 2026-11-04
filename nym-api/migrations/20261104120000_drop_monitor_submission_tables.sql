/*
 * Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

-- nothing writes these any more and no route reads them: the v2 monitor's statuses, read only
-- under a cargo feature nothing enabled, and the results the network monitor orchestrator used to
-- submit, which fed nym-api's own figure. The v1 monitor's tables stay, frozen, behind the
-- deprecated history and status routes.
DROP TABLE IF EXISTS mixnode_status_v2;
DROP TABLE IF EXISTS gateway_status_v2;
DROP TABLE IF EXISTS mixnode_details_v2;
DROP TABLE IF EXISTS gateway_details_v2;
DROP TABLE IF EXISTS nym_node_stress_testing_result;
DROP TABLE IF EXISTS nym_node_liveness_result;

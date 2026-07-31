/*
 * Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

-- Every ip address the node announced via its self-described endpoint, comma-separated.
-- A node may announce several addresses of either family; testruns rotate through them so
-- each announced address gets exercised rather than whichever one happened to be picked.
-- NULL until the node has been successfully queried (same semantics as mixnet_socket_address).
ALTER TABLE nym_node
    ADD COLUMN announced_ips TEXT;

-- The address handed out for the most recent test run assignment. Used purely as the rotation
-- pointer into announced_ips, so it advances even when a run never reports a result.
ALTER TABLE nym_node
    ADD COLUMN last_tested_ip TEXT;

-- The address that was actually tested by this run, as reported by the agent that performed it.
-- NULL for runs recorded before the orchestrator started tracking it.
ALTER TABLE testrun
    ADD COLUMN tested_address TEXT;

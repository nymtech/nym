// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Config {
    /// How often nodes should be polled for updates on their self-described endpoints
    pub(crate) described_node_refresh_interval: Duration,

    /// Maximum number of nodes queried concurrently during a node refresh cycle.
    pub(crate) number_of_concurrent_node_queries: usize,

    /// Timeout for querying a single node for its detailed information (ip addresses) (e.g. `10s`).
    pub(crate) node_info_query_timeout: Duration,

    /// Validity of geolocation data (e.g. 30 days) before new information is pushed to the contract
    pub(crate) geolocation_data_ttl: Duration,

    /// Short-lived lookup cache for repeated ip-info requests
    pub(crate) ip_info_lookup_cache_ttl: Duration,

    /// How often bonded nodes should be refreshed
    pub(crate) bonded_nodes_refresh_interval: Duration,

    /// How often should the geolocator check for expired geolocation data
    pub(crate) geolocation_expiration_polling_interval: Duration,
}

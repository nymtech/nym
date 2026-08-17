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

    /// Maximum number of addresses a node may announce before its details are rejected outright.
    ///
    /// The announced list is unverified input and every entry in it costs a metered lookup, so
    /// without a bound one node claiming a thousand addresses drains the provider allowance for
    /// the whole network. Rejected rather than truncated: taking the first few would let a node
    /// choose which of its addresses gets geolocated simply by ordering them.
    pub(crate) max_addresses_per_node: usize,

    /// Validity of geolocation data (e.g. 30 days) before new information is pushed to the contract
    pub(crate) geolocation_data_ttl: Duration,

    /// Short-lived lookup cache for repeated ip-info requests
    pub(crate) ip_info_lookup_cache_ttl: Duration,

    /// Maximum number of addresses handed to the lookup provider in a single request.
    ///
    /// The provider applies one flat timeout to a batch however large it is, so an entire sweep
    /// sent as one request either fits inside that budget or fails whole and submits nothing.
    /// Splitting it gives each part its own budget, and confines a part that fails to the nodes
    /// in it. It also bounds how long a sweep can hold the provider client, which the http
    /// handlers share.
    pub(crate) max_addresses_per_lookup: usize,

    /// How often bonded nodes should be refreshed
    pub(crate) bonded_nodes_refresh_interval: Duration,

    /// How often should the geolocator check for expired geolocation data
    pub(crate) geolocation_expiration_polling_interval: Duration,

    /// How long a node-signed re-test request stays valid after it was signed.
    ///
    /// Short by design: it is the window in which a captured request could be replayed before
    /// the monotonicity check is what stops it, and a node has no reason to sign one long
    /// before sending it.
    pub(crate) retest_request_validity_window: Duration,

    /// How many consecutive node-requested measurements may return an unchanged location before
    /// that node is put into cooldown.
    pub(crate) retest_burst_threshold: u32,

    /// How long a node that has spent its re-test allowance must wait.
    pub(crate) retest_burst_cooldown: Duration,

    /// Maximum number of nodes measured in a single sweep.
    ///
    /// Bounds the two bursts that are otherwise unbounded: a fresh agent, which has nothing on
    /// chain and so finds every node due at once, and the mass expiry that follows from it,
    /// since everything measured in one sweep expires in one sweep. Spreading the work also
    /// desynchronises those timestamps, so the burst flattens out over the first few TTLs.
    pub(crate) max_nodes_measured_per_sweep: usize,
}

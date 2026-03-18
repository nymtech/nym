// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Background task that periodically refreshes network topology and routing information.
//!
//! # Responsibilities
//!
//! - Fetches the current Nym node list from nym-api
//! - Resolves pending (unknown) IP addresses from ingress mixnet packets
//! - Updates routing filter with allowed/denied node lists
//! - Maintains Noise protocol key mappings
//! - Ensures minimally routable topology at startup
//!
//! # Refresh Strategy
//!
//! Two independent refresh cycles run in parallel:
//! 1. **Full refresh** (typically every 60s): Complete network state from nym-api
//! 2. **Pending check** (typically every 5s): Quick resolution of recently seen unknown IPs
//!
//! The pending check uses an optimized nym-api endpoint when available, falling back to
//! full refresh if the endpoint is not supported.

use crate::error::NymNodeError;
use crate::node::lp::directory::LpNodes;
use crate::node::nym_apis_client::NymApisClient;
use crate::node::routing_filter::network_filter::NetworkRoutingFilter;
use crate::node::shared_network::CachedNetwork;
use nym_node_metrics::prometheus_wrapper::{PROMETHEUS_METRICS, PrometheusMetric};
use nym_noise::config::{NoiseNetworkView, NoiseNode};
use nym_task::ShutdownToken;
use nym_topology::provider_trait::ToTopologyMetadata;
use nym_validator_client::ValidatorClientError;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use tokio::time::{Instant, interval, sleep};
use tracing::{debug, trace};

pub struct NetworkRefresher {
    config: NetworkRefresherConfig,
    client: NymApisClient,
    shutdown_token: ShutdownToken,

    network: CachedNetwork,
    routing_filter: NetworkRoutingFilter,
    noise_view: NoiseNetworkView,
    lp_nodes: LpNodes,
}

#[derive(Debug, Clone, Copy)]
pub struct NetworkRefresherConfig {
    full_refresh_interval: Duration,
    pending_check_interval: Duration,
    max_startup_waiting_period: Duration,
    min_mix_performance: u8,
}

impl NetworkRefresherConfig {
    pub fn new(
        full_refresh_interval: Duration,
        pending_check_interval: Duration,
        max_startup_waiting_period: Duration,
        min_mix_performance: u8,
    ) -> Self {
        Self {
            full_refresh_interval,
            pending_check_interval,
            max_startup_waiting_period,
            min_mix_performance,
        }
    }
}

impl NetworkRefresher {
    pub(crate) async fn initialise_new(
        config: NetworkRefresherConfig,
        client: NymApisClient,
        routing_filter: NetworkRoutingFilter,
        noise_view: NoiseNetworkView,
        shutdown_token: ShutdownToken,
    ) -> Result<Self, NymNodeError> {
        let mut this = NetworkRefresher {
            config,
            client,
            shutdown_token,
            network: CachedNetwork::new_empty(),
            routing_filter,
            noise_view,
            lp_nodes: Default::default(),
        };

        this.obtain_initial_network(
            config.max_startup_waiting_period,
            config.min_mix_performance,
        )
        .await?;
        Ok(this)
    }

    /// Attempt to resolve pending (unknown) IP addresses that were recently seen in packet traffic.
    ///
    /// # Algorithm
    ///
    /// 1. Collect all pending IPs that need resolution (lock required)
    /// 2. Short-circuit if all pending IPs are already in allowed/denied sets (race condition check)
    /// 3. Try optimised nym-api query: `query_nym_nodes_addresses(ips)` for bulk lookup
    ///    - If supported: Get immediate results, update allowed/denied sets, clear pending queue
    ///    - If not supported (404): Fall back to full network refresh
    ///
    /// # Performance
    ///
    /// The optimised query avoids fetching the entire network topology (~1000 nodes) when we only
    /// need to check a handful of IPs. This is crucial for minimising latency between when a new
    /// node joins and when it can successfully route packets.
    ///
    /// # Fallback Behaviour
    ///
    /// If nym-api doesn't support the optimised endpoint, we do a full refresh. This is acceptable
    /// because:
    /// - Full refresh is needed anyway for topology updates
    /// - The pending queue typically has <10 entries
    /// - This only affects older nym-api versions
    async fn inspect_pending(&mut self) {
        let to_resolve = self.routing_filter.pending.nodes().await;

        // no pending requests to resolve
        if to_resolve.is_empty() {
            return;
        }

        let mut allowed = self.routing_filter.allowed_nodes_copy();
        let mut denied = self.routing_filter.denied_nodes_copy();

        // short circuit: check if the pending nodes are not already resolved
        // (it could happen due to lack of full sync between pending lock and arcswap(s))
        if to_resolve
            .iter()
            .all(|p| allowed.contains(p) || denied.contains(p))
        {
            return;
        }

        // 1. attempt to use the new nym-api query to get information just by ips
        let nodes = to_resolve.into_iter().collect();
        if let Ok(res) = self.client.query_nym_nodes_addresses(nodes).await {
            for (ip, maybe_id) in res.existence {
                if maybe_id.is_some() {
                    allowed.insert(ip);
                } else {
                    denied.insert(ip);
                }
            }

            self.routing_filter.resolved.swap_allowed(allowed);
            self.routing_filter.resolved.swap_denied(denied);
            self.routing_filter.pending.clear().await;
            return;
        }

        // 2. we assume nym-api doesn't support that query yet - we have to do the full refresh
        self.refresh_network_nodes().await;
    }

    async fn refresh_network_nodes_inner(&mut self) -> Result<(), ValidatorClientError> {
        let rewarded_set = self.client.rewarded_set().await?;
        let res = self.client.current_nymnodes().await?;
        let nodes = res.nodes;
        let metadata = res.metadata;

        // collect all known/allowed nodes information
        let known_nodes = nodes
            .iter()
            .flat_map(|n| n.basic.ip_addresses.iter())
            .copied()
            .collect::<HashSet<_>>();

        let pending = self.routing_filter.pending.nodes().await;
        let mut current_denied = self.routing_filter.denied_nodes_copy();

        for allowed in &known_nodes {
            // if some node has become known, it should be removed from the denied set
            current_denied.remove(allowed);
        }

        // any pending node, if not present in the new set of allowed nodes, should be added in the denied set
        for pending_node in pending {
            if !known_nodes.contains(&pending_node) {
                current_denied.insert(pending_node);
            }
        }

        self.routing_filter.resolved.swap_allowed(known_nodes);
        self.routing_filter.resolved.swap_denied(current_denied);
        self.routing_filter.pending.clear().await;

        let noise_update_permit = self.noise_view.get_update_permit().await;
        let current_nodes = self.noise_view.all_nodes();

        // update noise Nodes
        let mut new_noise_nodes = HashMap::new();

        // 1. include all existing agents
        for (ip, node) in current_nodes {
            if !node.is_nym_node() {
                new_noise_nodes.insert(ip, node);
            }
        }

        // 2. iterate through the newly retrieved list of nym nodes
        for node in &nodes {
            let Some(noise_key) = node.x25519_noise_versioned_key else {
                continue;
            };
            let entry = NoiseNode::new_nym_node(noise_key);
            for ip_addr in &node.basic.ip_addresses {
                new_noise_nodes.insert(*ip_addr, entry.clone());
            }
        }
        self.noise_view
            .swap_view(noise_update_permit, new_noise_nodes);
        debug!("unimplemented: update LP nodes data - will work very similarly to noise nodes");

        let mut network_guard = self.network.inner.write().await;
        network_guard.topology_metadata = metadata.to_topology_metadata();
        network_guard.network_nodes = nodes;
        network_guard.rewarded_set = rewarded_set;

        Ok(())
    }

    async fn refresh_network_nodes(&mut self) {
        let timer =
            PROMETHEUS_METRICS.start_timer(PrometheusMetric::ProcessTopologyQueryResolutionLatency);

        if self.refresh_network_nodes_inner().await.is_err() {
            // don't use the histogram observation as some queries didn't complete
            if let Some(obs) = timer {
                obs.stop_and_discard();
            }
        }
    }

    /// Block until we obtain a minimally routable network topology at startup.
    ///
    /// # Startup Sequence
    ///
    /// 1. Query nym-api for full network state (nodes + rewarded set)
    /// 2. Check if topology has sufficient mixnodes in each layer for routing
    /// 3. If not routable: wait `STARTUP_REFRESH_INTERVAL` (30s) and retry
    /// 4. If still not routable after `max_startup_waiting_period`: return error and abort startup
    ///
    /// # Why Block Startup?
    ///
    /// We MUST have a routable topology before accepting packets, otherwise:
    /// - Packets would be dropped due to incomplete routing tables
    /// - The node would appear non-functional to the network
    /// - Internal service providers would be unable to construct return packets
    ///
    /// # Timeout Behavior
    ///
    /// If the network remains non-routable for too long, this indicates:
    /// - Network-wide outage (not enough mixnodes online)
    /// - nym-api connectivity issues
    /// - Configuration error (wrong nym-api URL)
    ///
    /// In any case, the node should NOT start packet processing.
    pub(crate) async fn obtain_initial_network(
        &mut self,
        max_startup_waiting_period: Duration,
        min_mix_performance: u8,
    ) -> Result<(), NymNodeError> {
        // make it configurable
        const STARTUP_REFRESH_INTERVAL: Duration = Duration::from_secs(30);

        let start = Instant::now();

        loop {
            self.refresh_network_nodes_inner()
                .await
                .map_err(|source| NymNodeError::InitialTopologyQueryFailure { source })?;

            let topology = self.network.network_topology(min_mix_performance).await;
            if topology.is_minimally_routable() {
                return Ok(());
            }

            if start.elapsed() > max_startup_waiting_period {
                return Err(NymNodeError::InitialTopologyTimeout);
            }

            sleep(STARTUP_REFRESH_INTERVAL).await;
        }
    }

    pub(crate) fn cached_network(&self) -> CachedNetwork {
        self.network.clone()
    }

    pub(crate) fn lp_nodes(&self) -> LpNodes {
        self.lp_nodes.clone()
    }

    pub(crate) async fn run(&mut self) {
        let mut full_refresh_interval = interval(self.config.full_refresh_interval);
        full_refresh_interval.reset();

        let mut pending_check_interval = interval(self.config.pending_check_interval);
        pending_check_interval.reset();

        loop {
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => {
                    trace!("NetworkRefresher: Received shutdown");
                    break;
                }
                _ = pending_check_interval.tick() => {
                    self.inspect_pending().await;
                }
                _ = full_refresh_interval.tick() => {
                    self.refresh_network_nodes().await;
                }
            }
        }
        trace!("NetworkRefresher: Exiting");
    }

    pub(crate) fn start(mut self) {
        tokio::spawn(async move { self.run().await });
    }
}

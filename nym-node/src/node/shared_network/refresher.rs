// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NymNodeError;
use crate::node::lp::directory::LpNodes;
use crate::node::nym_apis_client::NymApisClient;
use crate::node::routing_filter::network_filter::NetworkRoutingFilter;
use crate::node::shared_network::CachedNetwork;
use nym_http_api_client::UserAgent;
use nym_node_metrics::prometheus_wrapper::{PROMETHEUS_METRICS, PrometheusMetric};
use nym_noise::config::NoiseNetworkView;
use nym_task::ShutdownToken;
use nym_topology::EpochRewardedSet;
use nym_topology::provider_trait::ToTopologyMetadata;
use nym_validator_client::ValidatorClientError;
use nym_validator_client::nym_api::{NodesByAddressesResponse, SemiSkimmedNodesWithMetadata};
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tokio::time::{Instant, interval, sleep};
use tracing::log::error;
use tracing::{debug, trace, warn};
use url::Url;

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
        shutdown_token: ShutdownToken,
    ) -> Result<Self, NymNodeError> {
        let mut this = NetworkRefresher {
            config,
            client,
            shutdown_token,
            network: CachedNetwork::new_empty(),
            routing_filter,
            noise_view: NoiseNetworkView::new_empty(),
            lp_nodes: Default::default(),
        };

        this.obtain_initial_network(
            config.max_startup_waiting_period,
            config.min_mix_performance,
        )
        .await?;
        Ok(this)
    }

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

        // update noise Nodes
        let noise_nodes = nodes
            .iter()
            .filter(|n| n.x25519_noise_versioned_key.is_some())
            .flat_map(|n| {
                n.basic.ip_addresses.iter().map(|ip_addr| {
                    (
                        SocketAddr::new(*ip_addr, n.basic.mix_port),
                        #[allow(clippy::unwrap_used)]
                        n.x25519_noise_versioned_key.unwrap(), // SAFETY: we filtered out nodes where this option can be None
                    )
                })
            })
            .collect::<HashMap<_, _>>();
        self.noise_view.swap_view(noise_nodes);
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

    pub(crate) fn noise_view(&self) -> NoiseNetworkView {
        self.noise_view.clone()
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

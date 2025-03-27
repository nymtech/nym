// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NymNodeError;
use crate::node::mixnet::packet_forwarding::global::is_global_ip;
use arc_swap::ArcSwap;
use async_trait::async_trait;
use nym_gateway::node::UserAgent;
use nym_node_metrics::prometheus_wrapper::{PrometheusMetric, PROMETHEUS_METRICS};
use nym_task::ShutdownToken;
use nym_topology::node::RoutingNode;
use nym_topology::{EpochRewardedSet, NymTopology, Role, TopologyProvider};
use nym_validator_client::nym_api::NymApiClientExt;
use nym_validator_client::nym_nodes::{NodesByAddressesResponse, SkimmedNode};
use nym_validator_client::{NymApiClient, ValidatorClientError};
use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::log::error;
use tracing::{debug, trace, warn};
use url::Url;

pub(crate) trait RoutingFilter {
    fn should_route(&self, ip: IpAddr) -> bool;
}

#[derive(Debug, Copy, Clone, Default)]
pub(crate) struct OpenFilter;

impl RoutingFilter for OpenFilter {
    fn should_route(&self, _: IpAddr) -> bool {
        true
    }
}

impl RoutingFilter for NetworkRoutingFilter {
    fn should_route(&self, ip: IpAddr) -> bool {
        // only allow non-global ips on testnets
        if self.testnet_mode && !is_global_ip(&ip) {
            return true;
        }

        self.attempt_resolve(ip).should_route()
    }
}

#[derive(Clone)]
pub(crate) struct NetworkRoutingFilter {
    testnet_mode: bool,

    resolved: KnownNodes,

    // while this is technically behind a lock, it should not be called too often as once resolved it will
    // be present on the arcswap in either allowed or denied section
    pending: UnknownNodes,
}

impl NetworkRoutingFilter {
    fn new_empty(testnet_mode: bool) -> Self {
        NetworkRoutingFilter {
            testnet_mode,
            resolved: Default::default(),
            pending: Default::default(),
        }
    }

    pub(crate) fn attempt_resolve(&self, ip: IpAddr) -> Resolution {
        if self.resolved.inner.allowed.load().contains(&ip) {
            Resolution::Accept
        } else if self.resolved.inner.denied.load().contains(&ip) {
            Resolution::Deny
        } else {
            self.pending.try_insert(ip);
            Resolution::Unknown
        }
    }
}

#[derive(Clone, Default)]
struct UnknownNodes(Arc<RwLock<HashSet<IpAddr>>>);

impl UnknownNodes {
    fn try_insert(&self, ip: IpAddr) {
        // if we can immediately grab the lock to push it into the pending queue, amazing, let's do it
        // otherwise we can do it next time we see this ip
        // (if we can't hold the lock, it means it's being updated at this very moment which is actually a good thing)
        if let Ok(mut guard) = self.0.try_write() {
            guard.insert(ip);
        }
    }

    async fn clear(&self) {
        self.0.write().await.clear();
    }

    async fn nodes(&self) -> HashSet<IpAddr> {
        self.0.read().await.clone()
    }
}

// for now we don't care about keys, etc.
// we only want to know if given ip belongs to a known node
#[derive(Debug, Default, Clone)]
pub(crate) struct KnownNodes {
    inner: Arc<KnownNodesInner>,
}

#[derive(Debug, Default)]
struct KnownNodesInner {
    allowed: ArcSwap<HashSet<IpAddr>>,
    denied: ArcSwap<HashSet<IpAddr>>,
}

pub(crate) enum Resolution {
    Unknown,
    Deny,
    Accept,
}

impl From<bool> for Resolution {
    fn from(value: bool) -> Self {
        if value {
            Resolution::Accept
        } else {
            Resolution::Deny
        }
    }
}

impl Resolution {
    pub(crate) fn should_route(&self) -> bool {
        matches!(self, Resolution::Accept)
    }
}

impl KnownNodes {
    fn swap_allowed(&self, new: HashSet<IpAddr>) {
        self.inner.allowed.store(Arc::new(new))
    }

    fn swap_denied(&self, new: HashSet<IpAddr>) {
        self.inner.denied.store(Arc::new(new))
    }
}

struct NodesQuerier {
    client: NymApiClient,
    nym_api_urls: Vec<Url>,
    currently_used_api: usize,
}

impl NodesQuerier {
    fn use_next_nym_api(&mut self) {
        if self.nym_api_urls.len() == 1 {
            warn!("There's only a single nym API available - it won't be possible to use a different one");
            return;
        }

        self.currently_used_api = (self.currently_used_api + 1) % self.nym_api_urls.len();
        self.client
            .change_nym_api(self.nym_api_urls[self.currently_used_api].clone())
    }

    async fn rewarded_set(&mut self) -> Result<EpochRewardedSet, ValidatorClientError> {
        let res = self
            .client
            .get_current_rewarded_set()
            .await
            .inspect_err(|err| error!("failed to get current rewarded set: {err}"));

        if res.is_err() {
            self.use_next_nym_api()
        }
        res
    }

    async fn current_nymnodes(&mut self) -> Result<Vec<SkimmedNode>, ValidatorClientError> {
        let res = self
            .client
            .get_all_basic_nodes()
            .await
            .inspect_err(|err| error!("failed to get network nodes: {err}"));

        if res.is_err() {
            self.use_next_nym_api()
        }
        res
    }

    async fn query_for_info(
        &mut self,
        ips: Vec<IpAddr>,
    ) -> Result<NodesByAddressesResponse, ValidatorClientError> {
        let res = self
            .client
            .nym_api
            .nodes_by_addresses(ips)
            .await
            .inspect_err(|err| error!("failed to obtain node information: {err}"));

        if res.is_err() {
            self.use_next_nym_api()
        }
        Ok(res?)
    }
}

#[derive(Clone)]
pub struct CachedTopologyProvider {
    gateway_node: Arc<RoutingNode>,
    cached_network: CachedNetwork,
    min_mix_performance: u8,
}

impl CachedTopologyProvider {
    pub(crate) fn new(
        gateway_node: RoutingNode,
        cached_network: CachedNetwork,
        min_mix_performance: u8,
    ) -> Self {
        CachedTopologyProvider {
            gateway_node: Arc::new(gateway_node),
            cached_network,
            min_mix_performance,
        }
    }
}

#[async_trait]
impl TopologyProvider for CachedTopologyProvider {
    async fn get_new_topology(&mut self) -> Option<NymTopology> {
        let network_guard = self.cached_network.inner.read().await;
        let self_node = self.gateway_node.identity_key;

        let mut topology = NymTopology::new_empty(network_guard.rewarded_set.clone())
            .with_additional_nodes(network_guard.network_nodes.iter().filter(|node| {
                if node.supported_roles.mixnode {
                    node.performance.round_to_integer() >= self.min_mix_performance
                } else {
                    true
                }
            }));

        if !topology.has_node_details(self.gateway_node.node_id) {
            debug!("{self_node} didn't exist in topology. inserting it.",);
            topology.insert_node_details(self.gateway_node.as_ref().clone());
        }
        topology.force_set_active(self.gateway_node.node_id, Role::EntryGateway);

        Some(topology)
    }
}

#[derive(Clone)]
pub(crate) struct CachedNetwork {
    inner: Arc<RwLock<CachedNetworkInner>>,
}

impl CachedNetwork {
    fn new_empty() -> Self {
        CachedNetwork {
            inner: Arc::new(RwLock::new(CachedNetworkInner {
                rewarded_set: Default::default(),
                network_nodes: vec![],
            })),
        }
    }
}

struct CachedNetworkInner {
    rewarded_set: EpochRewardedSet,
    network_nodes: Vec<SkimmedNode>,
}

pub struct NetworkRefresher {
    querier: NodesQuerier,
    full_refresh_interval: Duration,
    pending_check_interval: Duration,
    shutdown_token: ShutdownToken,

    network: CachedNetwork,
    routing_filter: NetworkRoutingFilter,
}

impl NetworkRefresher {
    pub(crate) async fn initialise_new(
        testnet: bool,
        user_agent: UserAgent,
        nym_api_urls: Vec<Url>,
        full_refresh_interval: Duration,
        pending_check_interval: Duration,
        shutdown_token: ShutdownToken,
    ) -> Result<Self, NymNodeError> {
        let nym_api = nym_http_api_client::Client::builder(nym_api_urls[0].clone())?
            .no_hickory_dns()
            .with_user_agent(user_agent)
            .build()?;

        let mut this = NetworkRefresher {
            querier: NodesQuerier {
                client: NymApiClient { nym_api },
                nym_api_urls,
                currently_used_api: 0,
            },
            full_refresh_interval,
            pending_check_interval,
            shutdown_token,
            network: CachedNetwork::new_empty(),
            routing_filter: NetworkRoutingFilter::new_empty(testnet),
        };

        this.obtain_initial_network().await?;
        Ok(this)
    }

    fn allowed_nodes_copy(&self) -> HashSet<IpAddr> {
        self.routing_filter
            .resolved
            .inner
            .allowed
            .load_full()
            .as_ref()
            .clone()
    }

    fn denied_nodes_copy(&self) -> HashSet<IpAddr> {
        self.routing_filter
            .resolved
            .inner
            .denied
            .load_full()
            .as_ref()
            .clone()
    }

    async fn inspect_pending(&mut self) {
        let to_resolve = self.routing_filter.pending.nodes().await;

        // no pending requests to resolve
        if to_resolve.is_empty() {
            return;
        }

        let mut allowed = self.allowed_nodes_copy();
        let mut denied = self.denied_nodes_copy();

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
        if let Ok(res) = self.querier.query_for_info(nodes).await {
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
        let rewarded_set = self.querier.rewarded_set().await?;
        let nodes = self.querier.current_nymnodes().await?;

        // collect all known/allowed nodes information
        let known_nodes = nodes
            .iter()
            .flat_map(|n| n.ip_addresses.iter())
            .copied()
            .collect::<HashSet<_>>();

        let pending = self.routing_filter.pending.nodes().await;
        let mut current_denied = self.denied_nodes_copy();

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

        let mut network_guard = self.network.inner.write().await;
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

    pub(crate) async fn obtain_initial_network(&mut self) -> Result<(), NymNodeError> {
        self.refresh_network_nodes_inner()
            .await
            .map_err(|source| NymNodeError::InitialTopologyQueryFailure { source })
    }

    pub(crate) fn routing_filter(&self) -> NetworkRoutingFilter {
        self.routing_filter.clone()
    }

    pub(crate) fn cached_network(&self) -> CachedNetwork {
        self.network.clone()
    }

    pub(crate) async fn run(&mut self) {
        let mut full_refresh_interval = interval(self.full_refresh_interval);
        full_refresh_interval.reset();

        let mut pending_check_interval = interval(self.pending_check_interval);
        pending_check_interval.reset();

        while !self.shutdown_token.is_cancelled() {
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => {
                   trace!("NetworkRefresher: Received shutdown");
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

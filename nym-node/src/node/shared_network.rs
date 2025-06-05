// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NymNodeError;
use crate::node::key_rotation::active_keys::ActiveSphinxKeys;
use crate::node::routing_filter::network_filter::NetworkRoutingFilter;
use async_trait::async_trait;
use nym_crypto::asymmetric::ed25519;
use nym_gateway::node::UserAgent;
use nym_node_metrics::prometheus_wrapper::{PrometheusMetric, PROMETHEUS_METRICS};
use nym_noise::config::NoiseNetworkView;
use nym_task::ShutdownToken;
use nym_topology::node::RoutingNode;
use nym_topology::{
    EntryDetails, EpochRewardedSet, NodeId, NymTopology, NymTopologyMetadata, Role,
    TopologyProvider,
};
use nym_validator_client::nym_api::NymApiClientExt;
use nym_validator_client::nym_nodes::{
    NodesByAddressesResponse, SemiSkimmedNode, SemiSkimmedNodesWithMetadata,
};
use nym_validator_client::{NymApiClient, ValidatorClientError};
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, SocketAddr};
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::log::error;
use tracing::{debug, trace, warn};
use url::Url;

const LOCAL_NODE_ID: NodeId = 1234567890;

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

    async fn current_nymnodes(
        &mut self,
    ) -> Result<SemiSkimmedNodesWithMetadata, ValidatorClientError> {
        let res = self
            .client
            .get_all_expanded_nodes()
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

pub(crate) struct LocalGatewayNode {
    pub(crate) active_sphinx_keys: ActiveSphinxKeys,
    pub(crate) mix_host: SocketAddr,
    pub(crate) identity_key: ed25519::PublicKey,
    pub(crate) entry: EntryDetails,
}

impl LocalGatewayNode {
    pub(crate) fn to_routing_node(&self) -> RoutingNode {
        RoutingNode {
            node_id: LOCAL_NODE_ID,
            mix_host: self.mix_host,
            entry: Some(self.entry.clone()),
            identity_key: self.identity_key,
            sphinx_key: self.active_sphinx_keys.primary().deref().x25519_pubkey(),
            supported_roles: nym_topology::SupportedRoles {
                mixnode: false,
                mixnet_entry: true,
                mixnet_exit: true,
            },
        }
    }
}

#[derive(Clone)]
pub struct CachedTopologyProvider {
    gateway_node: Arc<LocalGatewayNode>,
    cached_network: CachedNetwork,
    min_mix_performance: u8,
}

impl CachedTopologyProvider {
    pub(crate) fn new(
        gateway_node: LocalGatewayNode,
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

        let mut topology = NymTopology::new(
            network_guard.topology_metadata,
            network_guard.rewarded_set.clone(),
            Vec::new(),
        )
        .with_additional_nodes(
            network_guard
                .network_nodes
                .iter()
                .map(|node| &node.basic)
                .filter(|node| {
                    if node.supported_roles.mixnode {
                        node.performance.round_to_integer() >= self.min_mix_performance
                    } else {
                        true
                    }
                }),
        );

        if !topology.has_node(self.gateway_node.identity_key) {
            debug!("{self_node} didn't exist in topology. inserting it.",);
            topology.insert_node_details(self.gateway_node.to_routing_node());
        }
        topology.force_set_active(LOCAL_NODE_ID, Role::EntryGateway);

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
                topology_metadata: Default::default(),
                network_nodes: vec![],
            })),
        }
    }
}

struct CachedNetworkInner {
    rewarded_set: EpochRewardedSet,
    topology_metadata: NymTopologyMetadata,
    network_nodes: Vec<SemiSkimmedNode>,
}

pub struct NetworkRefresher {
    querier: NodesQuerier,
    full_refresh_interval: Duration,
    pending_check_interval: Duration,
    shutdown_token: ShutdownToken,

    network: CachedNetwork,
    routing_filter: NetworkRoutingFilter,
    noise_view: NoiseNetworkView,
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
                client: NymApiClient::from(nym_api),
                nym_api_urls,
                currently_used_api: 0,
            },
            full_refresh_interval,
            pending_check_interval,
            shutdown_token,
            network: CachedNetwork::new_empty(),
            routing_filter: NetworkRoutingFilter::new_empty(testnet),
            noise_view: NoiseNetworkView::new_empty(),
        };

        this.obtain_initial_network().await?;
        Ok(this)
    }

    async fn inspect_pending(&mut self) {
        let to_resolve = self.routing_filter.pending.nodes().await;

        // no pending requests to resolve
        if to_resolve.is_empty() {
            return;
        }

        let mut allowed = self.routing_filter().allowed_nodes_copy();
        let mut denied = self.routing_filter().denied_nodes_copy();

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
        let res = self.querier.current_nymnodes().await?;
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

        //update noise Noise Nodes
        let noise_nodes = nodes
            .iter()
            .filter(|n| n.x25519_noise_versioned_key.is_some())
            .flat_map(|n| {
                n.basic.ip_addresses.iter().map(|ip_addr| {
                    (
                        SocketAddr::new(*ip_addr, n.basic.mix_port),
                        #[allow(clippy::unwrap_used)]
                        n.x25519_noise_versioned_key.unwrap(), // SAFETY : we filtered out nodes where this option can be None
                    )
                })
            })
            .collect::<HashMap<_, _>>();
        self.noise_view.swap_view(noise_nodes);

        let mut network_guard = self.network.inner.write().await;
        network_guard.topology_metadata =
            NymTopologyMetadata::new(metadata.rotation_id, metadata.absolute_epoch_id);
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

    pub(crate) fn noise_view(&self) -> NoiseNetworkView {
        self.noise_view.clone()
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

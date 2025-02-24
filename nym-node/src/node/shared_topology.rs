// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NymNodeError;
use arc_swap::ArcSwap;
use async_trait::async_trait;
use nym_gateway::node::UserAgent;
use nym_node_metrics::prometheus_wrapper::{PrometheusMetric, PROMETHEUS_METRICS};
use nym_topology::node::RoutingNode;
use nym_topology::{EpochRewardedSet, NymTopology, Role, TopologyProvider};
use nym_validator_client::nym_nodes::SkimmedNode;
use nym_validator_client::{NymApiClient, ValidatorClientError};
use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::Mutex;
use tracing::log::error;
use tracing::{debug, warn};
use url::Url;

// TODO: make it configurable
const MIN_MIX_PERFORMANCE: u8 = 80;

// for now we don't care about keys, etc.
// we only want to know if given ip belongs to a known node
#[derive(Debug, Default, Clone)]
pub(crate) struct KnownNodes {
    inner: Arc<KnownNodesInner>,
}

#[derive(Debug, Default)]
struct KnownNodesInner {
    nodes: ArcSwap<HashSet<IpAddr>>,
    // list of recently requested from nym-api
    // requests in transit, etc.
}

struct Pending {
    denied: HashSet<IpAddr>,
    in_transit: HashSet<IpAddr>,
    queued_up: HashSet<IpAddr>,
}

struct CachedTopology {
    topology: NymTopology,
    cached_at: OffsetDateTime,
    cache_ttl: Duration,
}

impl CachedTopology {
    // internal method to initialise the struct,
    // don't expose it as it's semi-undefined due to empty topology (without even epoch id information)
    fn new_empty(cache_ttl: Duration) -> Self {
        CachedTopology {
            topology: NymTopology::default(),
            cached_at: OffsetDateTime::UNIX_EPOCH,
            cache_ttl,
        }
    }

    fn replace(&mut self, topology: NymTopology) {
        self.topology = topology;
        self.cached_at = OffsetDateTime::now_utc();
    }

    fn get(&self) -> Option<&NymTopology> {
        if self.cached_at + self.cache_ttl > OffsetDateTime::now_utc() {
            Some(&self.topology)
        } else {
            None
        }
    }
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
    pub(crate) fn attempt_resolve(&self, ip: IpAddr) -> Resolution {
        self.inner.nodes.load().contains(&ip).into()
    }

    fn swap(&self, new: HashSet<IpAddr>) {
        self.inner.nodes.store(Arc::new(new))
    }
}

//
impl NymNodeTopologyProvider {
    /// Create new instance of NymNodeTopologyProvider with an initial network topology
    /// it returns an error if it fails to retrieve initial network information
    pub async fn initialise_new(
        gateway_node: RoutingNode,
        cache_ttl: Duration,
        user_agent: UserAgent,
        nym_api_urls: Vec<Url>,
    ) -> Result<NymNodeTopologyProvider, NymNodeError> {
        let this = NymNodeTopologyProvider {
            inner: Arc::new(NymNodeTopologyProviderInner {
                querier: Mutex::new(NodesQuerier {
                    client: NymApiClient::new_with_user_agent(nym_api_urls[0].clone(), user_agent),
                    nym_api_urls,
                    currently_used_api: 0,
                }),
                topology: Mutex::new(CachedTopology::new_empty(cache_ttl)),
                known_nodes: Default::default(),
                gateway_node,
            }),
        };

        this.inner
            .refresh_inner()
            .await
            .map_err(|source| NymNodeError::InitialTopologyQueryFailure { source })?;
        Ok(this)
    }

    pub(crate) fn known_nodes(&self) -> KnownNodes {
        self.inner.known_nodes.clone()
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

    async fn rewarded_set(&self) -> Result<EpochRewardedSet, ValidatorClientError> {
        self.client
            .get_current_rewarded_set()
            .await
            .inspect_err(|err| error!("failed to get current rewarded set: {err}"))
    }

    async fn current_nymnodes(&self) -> Result<Vec<SkimmedNode>, ValidatorClientError> {
        self.client
            .get_all_basic_nodes()
            .await
            .inspect_err(|err| error!("failed to get network nodes: {err}"))
    }

    async fn query_for_info(&self, ips: Vec<IpAddr>) -> ! {
        let _ = ips;
        todo!()
    }
}

struct NodesWrapper {
    querier: NodesQuerier,
    nodes: KnownNodes,
}

#[derive(Clone)]
pub struct NymNodeTopologyProvider {
    inner: Arc<NymNodeTopologyProviderInner>,
}

impl NymNodeTopologyProvider {
    async fn cached_topology(&self) -> Option<NymTopology> {
        self.inner.topology.lock().await.get().cloned()
    }
}

#[async_trait]
impl TopologyProvider for NymNodeTopologyProvider {
    async fn get_new_topology(&mut self) -> Option<NymTopology> {
        // check the cache
        if let Some(cached) = self.cached_topology().await {
            return Some(cached);
        }

        self.inner.refresh().await
    }
}

struct NymNodeTopologyProviderInner {
    querier: Mutex<NodesQuerier>,

    // NOTE: if we ever decide to serve this to external clients, it **HAS TO BE CHANGED**
    // because it's not the EXACT view of the network since we unconditionally include our node as an entry gateway
    // AND we filter out nodes based on performance (as it's used by IPR/NR/Auth)
    topology: Mutex<CachedTopology>,
    known_nodes: KnownNodes,

    gateway_node: RoutingNode,
}

impl NymNodeTopologyProviderInner {
    // every time we pull full topology also update the set of known nodes
    async fn refresh_inner(&self) -> Result<NymTopology, ValidatorClientError> {
        let querier = self.querier.lock().await;
        let rewarded_set = querier.rewarded_set().await?;
        let nodes = querier.current_nymnodes().await?;

        // collect all known nodes information
        let known_nodes = nodes
            .iter()
            .flat_map(|n| n.ip_addresses.iter())
            .copied()
            .collect();

        // build the topology
        let mut topology = NymTopology::new_empty(rewarded_set).with_additional_nodes(
            nodes
                .iter()
                .filter(|n| n.performance.round_to_integer() >= MIN_MIX_PERFORMANCE),
        );

        let self_node = self.gateway_node.identity_key;
        if !topology.has_node_details(self.gateway_node.node_id) {
            debug!("{self_node} didn't exist in topology. inserting it.",);
            topology.insert_node_details(self.gateway_node.clone());
        }
        topology.force_set_active(self.gateway_node.node_id, Role::EntryGateway);

        // update cache
        self.topology.lock().await.replace(topology.clone());

        // update known nodes
        self.known_nodes.swap(known_nodes);

        Ok(topology)
    }

    async fn refresh(&self) -> Option<NymTopology> {
        // the observation will be included on drop
        // note: it's ever so slightly biased by locking delay,
        // but I hope in the grand of scheme of things it's negligible
        let timer =
            PROMETHEUS_METRICS.start_timer(PrometheusMetric::ProcessTopologyQueryResolutionLatency);

        match self.refresh_inner().await {
            Ok(topology) => Some(topology),
            Err(err) => {
                // don't use the histogram observation as some queries didn't complete
                if let Some(obs) = timer {
                    obs.stop_and_discard();
                }

                warn!("failed to refresh network information: {err}");
                self.querier.lock().await.use_next_nym_api();
                None
            }
        }
    }
}

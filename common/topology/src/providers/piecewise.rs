// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//!
//!
#![warn(missing_docs)]

use crate::{EpochRewardedSet, NymTopology, RoutingNode, TopologyProvider};

use async_trait::async_trait;
use time::OffsetDateTime;
use tokio::sync::Mutex;
use tracing::{debug, warn};

use std::{
    cmp::min,
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

#[derive(Debug)]
pub struct Config {
    pub min_mixnode_performance: u8,
    pub min_gateway_performance: u8,
    pub use_extended_topology: bool,
    pub ignore_egress_epoch_role: bool,

    /// Minimum duration during which querying the topology will NOT attempt to re-fetch data, and
    /// will be served from cache.
    pub cache_ttl: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            min_mixnode_performance: 50,
            min_gateway_performance: 0,
            use_extended_topology: false,
            ignore_egress_epoch_role: true,
            cache_ttl: Self::DEFAULT_TOPOLOGY_CACHE_TTL,
        }
    }
}

impl From<nym_client_core_config_types::Topology> for Config {
    fn from(value: nym_client_core_config_types::Topology) -> Self {
        Config {
            min_mixnode_performance: value.minimum_mixnode_performance,
            min_gateway_performance: value.minimum_gateway_performance,
            use_extended_topology: value.use_extended_topology,
            ignore_egress_epoch_role: value.ignore_egress_epoch_role,
            cache_ttl: value.topology_refresh_rate,
        }
    }
}

impl Config {
    /// Default duration during which the topology will be reproduced from cache.
    pub const DEFAULT_TOPOLOGY_CACHE_TTL: Duration = Duration::from_secs(120);

    // if we're using 'extended' topology, filter the nodes based on the lowest set performance
    fn min_node_performance(&self) -> u8 {
        min(self.min_mixnode_performance, self.min_gateway_performance)
    }
}

/// Topology Provider build around a cached piecewise provider that uses the Nym API to
/// fetch changes and node details.
#[derive(Clone)]
pub struct NymTopologyProvider<M: PiecewiseTopologyProvider> {
    inner: Arc<Mutex<NymTopologyProviderInner<M>>>,
}

impl<M: PiecewiseTopologyProvider> NymTopologyProvider<M> {
    /// Construct a new thread safe Cached topology provider
    pub fn new(
        manager: M,
        config: Config,
        initial_topology: Option<NymTopology>,
    ) -> NymTopologyProvider<M> {
        let inner = NymTopologyProviderInner::new(config, manager, initial_topology);

        NymTopologyProvider {
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    /// Bypass the caching for the topology and force a check for the latest updates next time the
    /// topology is requested.
    pub async fn force_refresh(&self) {
        let mut guard = self.inner.lock().await;
        guard.cached_at = OffsetDateTime::UNIX_EPOCH;
    }

    /// Remove all stored topology state. The next time the topology is requested this will force a
    /// pull of all topology information.
    ///
    /// WARNING: This may be slow / require non-trivial bandwidth.
    pub async fn force_clear(&self) {
        let mut guard = self.inner.lock().await;
        guard.cached = None;
    }
}

#[async_trait]
impl<M: PiecewiseTopologyProvider> TopologyProvider for NymTopologyProvider<M> {
    async fn get_new_topology(&mut self) -> Option<NymTopology> {
        let mut guard = self.inner.lock().await;
        // check the cache
        if let Some(cached) = guard.get_current_compatible_topology().await {
            return Some(cached);
        }

        // not cached, or cache expired. try update.
        guard.update_cache().await;
        guard.get_current_compatible_topology().await
    }
}

struct NymTopologyProviderInner<M: PiecewiseTopologyProvider> {
    config: Config,

    cached: Option<NymTopology>,
    cached_at: OffsetDateTime,

    topology_manager: M,
}

impl<M: PiecewiseTopologyProvider> NymTopologyProviderInner<M> {
    pub fn new(
        config: impl Into<Config>,
        manager: M,
        initial_topology: Option<NymTopology>,
    ) -> Self {
        Self {
            config: config.into(),
            cached_at: OffsetDateTime::UNIX_EPOCH,
            cached: initial_topology,
            topology_manager: manager,
        }
    }

    fn cached_topology(&self) -> Option<NymTopology> {
        if let Some(cached_topology) = &self.cached {
            if self.cached_at + self.config.cache_ttl > OffsetDateTime::now_utc() {
                return Some(cached_topology.clone());
            }
        }

        None
    }

    async fn update_cache(&mut self) {
        if let Some(ref mut cached_topology) = self.cached {
            // get layer assignment map
            let response = self.topology_manager.get_layer_assignments().await;
            if response.is_none() {
                warn!("pulled layer assignments and got no response");
                self.cached_at = OffsetDateTime::now_utc();
                return;
            }

            let layer_assignments = response.unwrap();

            // Check if we already know about the epoch
            if cached_topology.rewarded_set.epoch_id == layer_assignments.epoch_id {
                debug!("pulled layer assignments, epoch already known");
                self.cached_at = OffsetDateTime::now_utc();
                return;
            }

            // get the set of node IDs
            let newly_assigned_node_ids = layer_assignments.assignment.all_ids();
            let new_id_set = HashSet::<u32>::from_iter(newly_assigned_node_ids);
            let known_id_set = HashSet::<u32>::from_iter(cached_topology.all_node_ids().copied());
            let unknown_node_ids: Vec<_> = new_id_set.difference(&known_id_set).copied().collect();

            // Pull node descriptors for unknown IDs
            let response = self
                .topology_manager
                .get_descriptor_batch(&unknown_node_ids[..])
                .await;

            // Add the new nodes to our cached topology
            if let Some(new_descriptors) = response {
                cached_topology.add_routing_nodes(new_descriptors.values());
            }
        } else {
            self.cached = self.topology_manager.get_full_topology().await;
        }

        self.cached_at = OffsetDateTime::now_utc();
    }

    /// Gets the current topology state using `Self::cached_topology` and then applies any filters
    /// defined in the provided Config.
    async fn get_current_compatible_topology(&mut self) -> Option<NymTopology> {
        let full_topology = self.cached_topology()?;

        let mut topology = NymTopology::new_empty(full_topology.rewarded_set().clone());

        if self.config.use_extended_topology {
            topology.add_additional_nodes(full_topology.all_nodes().filter(|n| {
                n.performance.round_to_integer() >= self.config.min_node_performance()
            }));

            return Some(full_topology);
        }

        topology.add_additional_nodes(
            full_topology.mixnodes().filter(|m| {
                m.performance.round_to_integer() >= self.config.min_mixnode_performance
            }),
        );
        topology.add_additional_nodes(
            full_topology.gateways().filter(|m| {
                m.performance.round_to_integer() >= self.config.min_gateway_performance
            }),
        );

        Some(topology)
    }
}

#[async_trait]
impl<P: PiecewiseTopologyProvider> TopologyProvider for NymTopologyProviderInner<P> {
    async fn get_new_topology(&mut self) -> Option<NymTopology> {
        self.get_current_compatible_topology().await
    }
}

/// Trait allowing construction and upkeep of a
#[async_trait]
pub trait PiecewiseTopologyProvider: Send {
    async fn get_full_topology(&mut self) -> Option<NymTopology>;

    async fn get_descriptor_batch(&mut self, ids: &[u32]) -> Option<HashMap<u32, RoutingNode>>;

    async fn get_layer_assignments(&mut self) -> Option<EpochRewardedSet>;
}

#[cfg(test)]
mod test {
    use std::net::SocketAddr;

    use nym_mixnet_contract_common::Percent;
    use nym_crypto::asymmetric::identity::PublicKey as IdentityPubkey;
    use nym_crypto::asymmetric::encryption::PublicKey as SphinxPubkey;
    use super::*;
    use crate::SupportedRoles;

    #[derive(Clone)]
    struct PassthroughPiecewiseTopologyProvider {
        topo: NymTopology,
    }

    #[async_trait]
    impl PiecewiseTopologyProvider for PassthroughPiecewiseTopologyProvider {
        async fn get_full_topology(&mut self) -> Option<NymTopology> {
            Some(self.topo.clone())
        }

        async fn get_descriptor_batch(&mut self, ids: &[u32]) -> Option<HashMap<u32, RoutingNode>> {
            let mut nodes = HashMap::new();
            ids.iter().for_each(|id| {
                if let Some(node) = self.topo.node_details.get(id) {
                    nodes.insert(*id, node.clone());
                }
            });

            Some(nodes)
        }

        async fn get_layer_assignments(&mut self) -> Option<EpochRewardedSet> {
            return Some(self.topo.rewarded_set.clone().into());
        }
    }

    #[tokio::test]
    async fn test_topology_provider() -> Result<(), Box<dyn std::error::Error>> {
        let mut topo_mgr = PassthroughPiecewiseTopologyProvider {
            topo: NymTopology::default(),
        };

        let mut topo_provider =
            NymTopologyProviderInner::new(Config::default(), topo_mgr.clone(), None);

        // No initial topology was provided, No update has run yet, None should be returned
        assert_eq!(topo_provider.cached_topology(), None);

        // force an update of the cached topology
        topo_provider.update_cache().await;

        let topo = topo_provider.cached_topology();
        assert!(topo.is_some());
        let topo = topo.unwrap();
        assert!(topo.is_empty());
        
        // create a change in the manager to make sure it is propogated to the provider cache on update
        topo_mgr.topo.rewarded_set.epoch_id += 1;
        topo_mgr.topo.rewarded_set.entry_gateways = HashSet::from([123]);
        assert_eq!(topo_mgr.topo.node_details.insert(123, fake_node(123)), None);
        topo_provider.topology_manager = topo_mgr.clone();

        // force an update of the cached topology
        topo_provider.update_cache().await;

        let topo = topo_provider.cached_topology();
        assert!(topo.is_some());
        let topo1 = topo.unwrap();
        assert!(!topo1.is_empty());
        assert!(topo1.node_details.contains_key(&123));

        // try forcing an update even though the epoch has not changed. Should result in no change
        topo_provider.update_cache().await;
        let topo2 = topo_provider.cached_topology().unwrap();
        assert_eq!(topo1, topo2);

        Ok(())
    }

    #[tokio::test]
    async fn test_topology_provider_by_trait() -> Result<(), Box<dyn std::error::Error>> {
        let topo_mgr = PassthroughPiecewiseTopologyProvider {
            topo: NymTopology::default(),
        };

        let _topo_provider =
            NymTopologyProviderInner::new(Config::default(), topo_mgr.clone(), None);

        Ok(())
    }

    fn fake_node(node_id: u32) -> RoutingNode {
        RoutingNode {
            node_id,
            mix_host: "127.0.0.1:2345".parse().unwrap(),
            entry: None,
            identity_key: IdentityPubkey::from_bytes(&[0u8; 32][..]).unwrap(),
            sphinx_key: SphinxPubkey::from_bytes(&[0u8; 32][..]).unwrap(),
            supported_roles: SupportedRoles {
                mixnode: true,
                mixnet_entry: true,
                mixnet_exit: true,
            },
            performance: Percent::hundred(),
        }
    }
}

// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//!
//!
#![warn(missing_docs)]

use crate::{EpochRewardedSet, NymTopology, Role, RoutingNode, TopologyProvider};

use async_trait::async_trait;
use time::OffsetDateTime;
use tokio::sync::Mutex;

use std::{cmp::min, collections::HashMap, sync::Arc, time::Duration};

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
		todo!("pull layer assignments and then batch");
        // let updated_cache = self.get_new_topology().await?;

        // self.cached_at = OffsetDateTime::now_utc();
        // self.cached = Some(updated_cache.clone());

        // Some(updated_cache)
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

		topology.add_additional_nodes(full_topology.mixnodes().filter(|m| {
			m.performance.round_to_integer() >= self.config.min_mixnode_performance
		}));
		topology.add_additional_nodes(full_topology.gateways().filter(|m| {
			m.performance.round_to_integer() >= self.config.min_gateway_performance
		}));
		
        Some(topology)
    }
}

#[async_trait]
impl<P: PiecewiseTopologyProvider> TopologyProvider for NymTopologyProviderInner<P> {
    async fn get_new_topology(&mut self) -> Option<NymTopology> {
        self.get_current_compatible_topology().await
    }
}

///
///
/// This is intentionally private, such that we can modify it at any time in the future.
#[async_trait]
pub trait PiecewiseTopologyProvider: Send {
    async fn get_full_topology(&mut self) -> Option<NymTopology>;

    async fn get_descriptor_batch(&mut self, ids: &[u32]) -> Option<HashMap<u32, RoutingNode>>;

    async fn get_layer_assignments(&mut self) -> Option<EpochRewardedSet>;
}


#[cfg(test)]
mod test {
    use super::*;

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
            return Some(self.topo.rewarded_set.clone().into())
        }
    }

    #[tokio::test]
    async fn test_topology_provider() -> Result<(), Box<dyn std::error::Error>> {
        let topo_mgr = PassthroughPiecewiseTopologyProvider {
            topo: NymTopology::default(),
        };

        let topo_provider = NymTopologyProviderInner::new(Config::default(), topo_mgr, None);

        Ok(())
    }
}

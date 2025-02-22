// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//!
//!
#![warn(missing_docs)]

use async_trait::async_trait;
use log::{debug, error, warn};
use nym_topology::{EpochRewardedSet, NymTopology, Role, RoutingNode, TopologyProvider};
use nym_validator_client::UserAgent;
use rand::{prelude::SliceRandom, thread_rng};
use time::OffsetDateTime;
use tokio::sync::Mutex;
use url::Url;

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
pub struct NymTopologyProvider {
    inner: Arc<Mutex<NymTopologyProviderInner<NymApiTopologyManager>>>,
}

impl NymTopologyProvider {
    pub fn new(
        user_agent: UserAgent,
        nym_api_urls: Vec<Url>,
        config: Config,
        initial_topology: Option<NymTopology>,
    ) -> NymTopologyProvider {
        let manager = NymApiTopologyManager::new(nym_api_urls, Some(user_agent));
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
impl TopologyProvider for NymTopologyProvider {
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

struct NymTopologyProviderInner<P: TopologyManager> {
    config: Config,

    cached: Option<NymTopology>,
    cached_at: OffsetDateTime,

    topology_manager: P,
}

impl<P: TopologyManager> NymTopologyProviderInner<P> {
    pub fn new(
        config: impl Into<Config>,
        manager: P,
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
impl<P: TopologyManager> TopologyProvider for NymTopologyProviderInner<P> {
    async fn get_new_topology(&mut self) -> Option<NymTopology> {
        self.get_current_compatible_topology().await
    }
}

///
///
/// This is intentionally private, such that we can modify it at any time in the future.
#[async_trait]
pub (crate) trait TopologyManager: Send {
    async fn get_full_topology(&mut self) -> Option<NymTopology>;

    async fn get_descriptor_batch(&mut self, ids: &[u32]) -> Option<HashMap<u32, RoutingNode>>;

    async fn get_layer_assignments(&mut self) -> Option<EpochRewardedSet>;
}

struct NymApiTopologyManager {
    validator_client: nym_validator_client::client::NymApiClient,
    nym_api_urls: Vec<Url>,
    currently_used_api: usize,
}

impl NymApiTopologyManager {
    fn new(mut nym_api_urls: Vec<Url>, user_agent: Option<UserAgent>) -> Self {
        nym_api_urls.shuffle(&mut thread_rng());

        let validator_client = if let Some(user_agent) = user_agent {
            nym_validator_client::client::NymApiClient::new_with_user_agent(
                nym_api_urls[0].clone(),
                user_agent,
            )
        } else {
            nym_validator_client::client::NymApiClient::new(nym_api_urls[0].clone())
        };

        Self {
            validator_client,
            nym_api_urls,
            currently_used_api: 0,
        }
    }

    fn use_next_nym_api(&mut self) {
        if self.nym_api_urls.len() == 1 {
            warn!("There's only a single nym API available - it won't be possible to use a different one");
            return;
        }

        self.currently_used_api = (self.currently_used_api + 1) % self.nym_api_urls.len();
        self.validator_client
            .change_nym_api(self.nym_api_urls[self.currently_used_api].clone())
    }
}

#[async_trait]
impl TopologyManager for NymApiTopologyManager {
    async fn get_full_topology(&mut self) -> Option<NymTopology> {
        let layer_assignments = self.get_layer_assignments().await?;

        let mut topology = NymTopology::new_empty(layer_assignments);

        let all_nodes = self
            .validator_client
            .get_all_basic_nodes()
            .await
            .inspect_err(|err| {
                self.use_next_nym_api();
                error!("failed to get network nodes: {err}");
            })
            .ok()?;

        debug!("there are {} nodes on the network", all_nodes.len());
        topology.add_additional_nodes(all_nodes.iter());

        if !topology.is_minimally_routable() {
            error!("the current filtered active topology can't be used to construct any packets");
            return None;
        }

        Some(topology)
    }

    async fn get_descriptor_batch(&mut self, ids: &[u32]) -> Option<HashMap<u32, RoutingNode>> {
        todo!("blocking on node batch endpoint")
    }

    async fn get_layer_assignments(&mut self) -> Option<EpochRewardedSet> {
        self.validator_client
            .get_current_rewarded_set()
            .await
            .inspect_err(|err| {
                self.use_next_nym_api();
                error!("failed to get current rewarded set: {err}");
            })
            .ok()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    struct TestTopoManager {
        topo: NymTopology,
    }

    #[async_trait]
    impl TopologyManager for TestTopoManager {
        async fn get_full_topology(&mut self) -> Option<NymTopology> {
            None
        }

        async fn get_descriptor_batch(&mut self, ids: &[u32]) -> Option<HashMap<u32, RoutingNode>> {
            None
        }

        async fn get_layer_assignments(&mut self) -> Option<EpochRewardedSet> {
            None
        }
    }

    #[tokio::test]
    async fn test_topology_provider() -> Result<(), Box<dyn std::error::Error>> {
        let topo_mgr = TestTopoManager {
            topo: NymTopology::default(),
        };

        let topo_provider = NymTopologyProviderInner::new(Config::default(), topo_mgr, None);

        Ok(())
    }
}

// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use nym_mixnet_contract_common::EpochRewardedSet;
use nym_topology::provider_trait::{ToTopologyMetadata, TopologyProvider};
use nym_topology::NymTopology;
use nym_validator_client::nym_api::NymApiClientExt;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use std::cmp::min;
use tracing::{debug, error, warn};
use url::Url;

#[derive(Debug)]
pub struct Config {
    pub min_mixnode_performance: u8,
    pub min_gateway_performance: u8,
    pub use_extended_topology: bool,
    pub ignore_egress_epoch_role: bool,
}

impl From<nym_client_core_config_types::Topology> for Config {
    fn from(value: nym_client_core_config_types::Topology) -> Self {
        Config {
            min_mixnode_performance: value.minimum_mixnode_performance,
            min_gateway_performance: value.minimum_gateway_performance,
            use_extended_topology: value.use_extended_topology,
            ignore_egress_epoch_role: value.ignore_egress_epoch_role,
        }
    }
}

impl Config {
    // if we're using 'extended' topology, filter the nodes based on the lowest set performance
    fn min_node_performance(&self) -> u8 {
        min(self.min_mixnode_performance, self.min_gateway_performance)
    }
}

pub struct NymApiTopologyProvider {
    config: Config,

    validator_client: nym_http_api_client::Client,
    nym_api_urls: Vec<Url>,
    currently_used_api: usize,
    use_bincode: bool,
}

impl NymApiTopologyProvider {
    pub fn new(
        config: impl Into<Config>,
        mut nym_api_urls: Vec<Url>,
        validator_client: nym_http_api_client::Client,
    ) -> Self {
        nym_api_urls.shuffle(&mut thread_rng());
        let mut provider = NymApiTopologyProvider {
            config: config.into(),
            validator_client,
            nym_api_urls,
            currently_used_api: 0,
            use_bincode: true,
        };
        // Set all API URLs - the client will try them in order with automatic failover
        provider.validator_client.change_base_urls(
            provider
                .nym_api_urls
                .iter()
                .map(|u| u.clone().into())
                .collect(),
        );
        provider
    }

    pub fn disable_bincode(&mut self) {
        self.use_bincode = false;
        // Note: The unified client doesn't support toggling bincode after creation.
        // This would require recreating the client without bincode.
        // For now, we'll track the preference but it won't take effect.
        warn!("Disabling bincode on existing client is not currently supported");
    }

    fn use_next_nym_api(&mut self) {
        if self.nym_api_urls.len() == 1 {
            warn!("There's only a single nym API available - it won't be possible to use a different one");
            return;
        }

        self.currently_used_api = (self.currently_used_api + 1) % self.nym_api_urls.len();
        
        // Provide all URLs starting from the next one in rotation order
        // This enables automatic failover to other endpoints
        let rotated_urls: Vec<_> = self.nym_api_urls
            .iter()
            .cycle()
            .skip(self.currently_used_api)
            .take(self.nym_api_urls.len())
            .map(|u| u.clone().into())
            .collect();
        
        self.validator_client.change_base_urls(rotated_urls)
    }

    async fn get_current_compatible_topology(&mut self) -> Option<NymTopology> {
        let rewarded_set_fut = self.validator_client.get_current_rewarded_set();

        let topology = if self.config.use_extended_topology {
            let all_nodes_fut = self.validator_client.get_all_basic_nodes_with_metadata();

            // Join rewarded_set_fut and all_nodes_fut concurrently
            let (rewarded_set, all_nodes_res) = futures::try_join!(rewarded_set_fut, all_nodes_fut)
                .inspect_err(|err| error!("failed to get network nodes: {err}"))
                .ok()?;

            let metadata = all_nodes_res.metadata;
            let all_nodes = all_nodes_res.nodes;

            debug!(
                "there are {} nodes on the network (before filtering)",
                all_nodes.len()
            );
            let nodes_filtered = all_nodes
                .into_iter()
                .filter(|n| n.performance.round_to_integer() >= self.config.min_node_performance())
                .collect::<Vec<_>>();

            let epoch_rewarded_set: EpochRewardedSet = rewarded_set.into();
            NymTopology::new(
                metadata.to_topology_metadata(),
                epoch_rewarded_set,
                Vec::new(),
            )
            .with_skimmed_nodes(&nodes_filtered)
        } else {
            // if we're not using extended topology, we're only getting active set mixnodes and gateways

            let mixnodes_fut = self
                .validator_client
                .get_all_basic_active_mixing_assigned_nodes_with_metadata();

            // TODO: we really should be getting ACTIVE gateways only
            let gateways_fut = self
                .validator_client
                .get_all_basic_entry_assigned_nodes_with_metadata();

            let (rewarded_set, mixnodes_res, gateways_res) =
                futures::try_join!(rewarded_set_fut, mixnodes_fut, gateways_fut)
                    .inspect_err(|err| {
                        error!("failed to get network nodes: {err}");
                    })
                    .ok()?;

            let metadata = mixnodes_res.metadata;
            let mixnodes = mixnodes_res.nodes;

            if !gateways_res.metadata.consistency_check(&metadata) {
                warn!("inconsistent nodes metadata between mixnodes and gateways calls! {metadata:?} and {:?}", gateways_res.metadata);
                return None;
            }

            let gateways = gateways_res.nodes;

            debug!(
                "there are {} mixnodes and {} gateways in total (before performance filtering)",
                mixnodes.len(),
                gateways.len()
            );

            let mut nodes = Vec::new();
            for mix in mixnodes {
                if mix.performance.round_to_integer() >= self.config.min_mixnode_performance {
                    nodes.push(mix)
                }
            }
            for gateway in gateways {
                if gateway.performance.round_to_integer() >= self.config.min_gateway_performance {
                    nodes.push(gateway)
                }
            }

            let epoch_rewarded_set: EpochRewardedSet = rewarded_set.into();
            NymTopology::new(
                metadata.to_topology_metadata(),
                epoch_rewarded_set,
                Vec::new(),
            )
            .with_skimmed_nodes(&nodes)
        };

        if !topology.is_minimally_routable() {
            error!("the current filtered active topology can't be used to construct any packets");
            return None;
        }

        Some(topology)
    }
}

// hehe, wasm
#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl TopologyProvider for NymApiTopologyProvider {
    async fn get_new_topology(&mut self) -> Option<NymTopology> {
        let Some(topology) = self.get_current_compatible_topology().await else {
            self.use_next_nym_api();
            return None;
        };
        Some(topology)
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
impl TopologyProvider for NymApiTopologyProvider {
    async fn get_new_topology(&mut self) -> Option<NymTopology> {
        let Some(topology) = self.get_current_compatible_topology().await else {
            self.use_next_nym_api();
            return None;
        };
        Some(topology)
    }
}

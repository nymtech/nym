// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use log::{debug, error};
use nym_topology::provider_trait::TopologyProvider;
use nym_topology::NymTopology;
use nym_validator_client::{NymApiClient, UserAgent};
use std::cmp::min;
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

    validator_client: nym_validator_client::client::NymApiClient,
}

impl NymApiTopologyProvider {
    pub fn new(
        config: impl Into<Config>,
        nym_api_urls: Vec<Url>,
        user_agent: Option<UserAgent>,
    ) -> Self {
        let urls = nym_api_urls.iter().map(Into::into).collect();
        let client = nym_http_api_client::ClientBuilder::new_with_urls(urls)
            .with_user_agent(user_agent)
            .with_retries(3)
            .build::<&str>()
            .expect("failed to create NymApiClient");
        let validator_client = NymApiClient::new_with_client(client);

        NymApiTopologyProvider {
            config: config.into(),
            validator_client,
        }
    }

    pub fn disable_bincode(&mut self) {
        self.validator_client.use_bincode = false;
    }

    async fn get_current_compatible_topology(&mut self) -> Option<NymTopology> {
        let rewarded_set_fut = self.validator_client.get_current_rewarded_set();

        let topology = if self.config.use_extended_topology {
            let all_nodes_fut = self.validator_client.get_all_basic_nodes();

            // Join rewarded_set_fut and all_nodes_fut concurrently
            let (rewarded_set, all_nodes) = futures::try_join!(rewarded_set_fut, all_nodes_fut)
                .inspect_err(|err| error!("failed to get network nodes: {err}"))
                .ok()?;

            debug!(
                "there are {} nodes on the network (before filtering)",
                all_nodes.len()
            );
            let mut topology = NymTopology::new_empty(rewarded_set);
            topology.add_additional_nodes(all_nodes.iter().filter(|n| {
                n.performance.round_to_integer() >= self.config.min_node_performance()
            }));

            topology
        } else {
            // if we're not using extended topology, we're only getting active set mixnodes and gateways

            let mixnodes_fut = self
                .validator_client
                .get_all_basic_active_mixing_assigned_nodes();

            // TODO: we really should be getting ACTIVE gateways only
            let gateways_fut = self.validator_client.get_all_basic_entry_assigned_nodes();

            let (rewarded_set, mixnodes, gateways) =
                futures::try_join!(rewarded_set_fut, mixnodes_fut, gateways_fut)
                    .inspect_err(|err| {
                        error!("failed to get network nodes: {err}");
                    })
                    .ok()?;

            debug!(
                "there are {} mixnodes and {} gateways in total (before performance filtering)",
                mixnodes.len(),
                gateways.len()
            );

            let mut topology = NymTopology::new_empty(rewarded_set);
            topology.add_additional_nodes(mixnodes.iter().filter(|m| {
                m.performance.round_to_integer() >= self.config.min_mixnode_performance
            }));
            topology.add_additional_nodes(gateways.iter().filter(|m| {
                m.performance.round_to_integer() >= self.config.min_gateway_performance
            }));

            topology
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
            return None;
        };
        Some(topology)
    }
}

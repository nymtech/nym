// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use log::{debug, error, warn};
use nym_topology::provider_trait::TopologyProvider;
use nym_topology::NymTopology;
use nym_validator_client::UserAgent;
use rand::prelude::SliceRandom;
use rand::thread_rng;
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
    nym_api_urls: Vec<Url>,
    currently_used_api: usize,
}

impl NymApiTopologyProvider {
    pub fn new(
        config: impl Into<Config>,
        mut nym_api_urls: Vec<Url>,
        user_agent: Option<UserAgent>,
    ) -> Self {
        nym_api_urls.shuffle(&mut thread_rng());

        let validator_client = if let Some(user_agent) = user_agent {
            nym_validator_client::client::NymApiClient::new_with_user_agent(
                nym_api_urls[0].clone(),
                user_agent,
            )
        } else {
            nym_validator_client::client::NymApiClient::new(nym_api_urls[0].clone())
        };

        NymApiTopologyProvider {
            config: config.into(),
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

    async fn get_current_compatible_topology(&mut self) -> Option<NymTopology> {
        let rewarded_set = self
            .validator_client
            .get_current_rewarded_set()
            .await
            .inspect_err(|err| error!("failed to get current rewarded set: {err}"))
            .ok()?;

        let mut topology = NymTopology::new_empty(rewarded_set);

        if self.config.use_extended_topology {
            let all_nodes = self
                .validator_client
                .get_all_basic_nodes()
                .await
                .inspect_err(|err| error!("failed to get network nodes: {err}"))
                .ok()?;

            debug!(
                "there are {} nodes on the network (before filtering)",
                all_nodes.len()
            );
            topology.add_additional_nodes(all_nodes.iter().filter(|n| {
                n.performance.round_to_integer() >= self.config.min_node_performance()
            }));
        } else {
            // if we're not using extended topology, we're only getting active set mixnodes and gateways

            let mixnodes = self
                .validator_client
                .get_all_basic_active_mixing_assigned_nodes()
                .await
                .inspect_err(|err| error!("failed to get network mixnodes: {err}"))
                .ok()?;

            // TODO: we really should be getting ACTIVE gateways only
            let gateways = self
                .validator_client
                .get_all_basic_entry_assigned_nodes()
                .await
                .inspect_err(|err| error!("failed to get network gateways: {err}"))
                .ok()?;

            debug!(
                "there are {} mixnodes and {} gateways in total (before performance filtering)",
                mixnodes.len(),
                gateways.len()
            );

            topology.add_additional_nodes(mixnodes.iter().filter(|m| {
                m.performance.round_to_integer() >= self.config.min_mixnode_performance
            }));
            topology.add_additional_nodes(gateways.iter().filter(|m| {
                m.performance.round_to_integer() >= self.config.min_gateway_performance
            }));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn foomp() -> anyhow::Result<()> {
        let provider = NymApiTopologyProvider::new(
            Config {
                min_mixnode_performance: 0,
                min_gateway_performance: 0,
                use_extended_topology: false,
                ignore_egress_epoch_role: false,
            },
            vec!["https://validator.nymtech.net/api/".parse()?],
            None,
        );

        let a = provider
            .validator_client
            .get_current_rewarded_set()
            .await
            .unwrap();

        println!("{a:#?}");

        panic!("");
        Ok(())
    }
}

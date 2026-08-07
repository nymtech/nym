// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use nym_mixnet_contract_common::EpochRewardedSet;
use nym_topology::NymTopology;
use nym_topology::provider_trait::{ToTopologyMetadata, TopologyProvider};
use nym_validator_client::nym_api::NymApiClientExt;
use nym_validator_client::nym_api::error::NymAPIError;
use std::cmp::min;
use tracing::{debug, error, warn};

#[derive(Debug, Copy, Clone)]
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
    use_bincode: bool,
}

impl NymApiTopologyProvider {
    pub fn new(config: impl Into<Config>, validator_client: nym_http_api_client::Client) -> Self {
        Self {
            config: config.into(),
            validator_client,
            use_bincode: true,
        }
    }

    pub fn disable_bincode(&mut self) {
        self.use_bincode = false;
        // Note: The unified client doesn't support toggling bincode after creation.
        // This would require recreating the client without bincode.
        // For now, we'll track the preference but it won't take effect.
        warn!("Disabling bincode on existing client is not currently supported");
    }

    async fn try_get_extended_topology(&mut self) -> Result<NymTopology, NymAPIError> {
        let rewarded_set_fut = self.validator_client.get_current_rewarded_set();
        let all_nodes_fut = self.validator_client.get_all_basic_nodes_with_metadata();

        // Join rewarded_set_fut and all_nodes_fut concurrently
        let (rewarded_set, all_nodes_res) = futures::try_join!(rewarded_set_fut, all_nodes_fut)
            .inspect_err(|err| {
                error!("failed to get network nodes: {err}");
            })?;

        let metadata = all_nodes_res.metadata;
        let all_nodes = all_nodes_res.nodes;

        if rewarded_set.epoch_id != metadata.absolute_epoch_id {
            // while technically the requests did succeed, they still returned data across epochs
            // so the responses were internally inconsistent
            return Err(NymAPIError::InternalResponseInconsistency {
                url: self.validator_client.current_url().clone().into(),
                details: format!(
                    "retrieved rewarded set information for different epoch than the nodes details {} and {}",
                    rewarded_set.epoch_id, metadata.absolute_epoch_id
                ),
            });
        }

        debug!(
            "there are {} nodes on the network (before filtering)",
            all_nodes.len()
        );
        let nodes_filtered = all_nodes
            .into_iter()
            .filter(|n| n.performance.round_to_integer() >= self.config.min_node_performance())
            .collect::<Vec<_>>();

        let epoch_rewarded_set: EpochRewardedSet = rewarded_set.into();
        Ok(NymTopology::new(
            metadata.to_topology_metadata(),
            epoch_rewarded_set,
            Vec::new(),
        )
        .with_skimmed_nodes(&nodes_filtered))
    }

    async fn try_get_active_topology(&mut self) -> Result<NymTopology, NymAPIError> {
        let rewarded_set_fut = self.validator_client.get_current_rewarded_set();

        let mixnodes_fut = self
            .validator_client
            .get_all_basic_active_mixing_assigned_nodes_with_metadata();

        // TODO: we really should be getting ACTIVE gateways only
        let gateways_fut = self
            .validator_client
            .get_all_basic_entry_assigned_nodes_with_metadata();

        let (rewarded_set, mixnodes_res, gateways_res) =
            futures::try_join!(rewarded_set_fut, mixnodes_fut, gateways_fut).inspect_err(
                |err| {
                    error!("failed to get network nodes: {err}");
                },
            )?;

        let metadata = mixnodes_res.metadata;
        let mixnodes = mixnodes_res.nodes;

        if !gateways_res.metadata.consistency_check(&metadata) {
            let msg = format!(
                "inconsistent nodes metadata between mixnodes and gateways calls! {metadata:?} and {:?}",
                gateways_res.metadata
            );
            warn!("{msg}");

            // while technically the requests did succeed, they still returned data across epochs
            // so the responses were internally inconsistent
            return Err(NymAPIError::InternalResponseInconsistency {
                url: self.validator_client.current_url().clone().into(),
                details: msg,
            });
        }

        // finally, compare epoch for the rewarded set and the nodes
        // (we might have got rewarded set for epoch 123, but mixnodes AND gateways for 124
        if rewarded_set.epoch_id != metadata.absolute_epoch_id {
            return Err(NymAPIError::InternalResponseInconsistency {
                url: self.validator_client.current_url().clone().into(),
                details: format!(
                    "retrieved rewarded set information for different epoch than the nodes details {} and {}",
                    rewarded_set.epoch_id, metadata.absolute_epoch_id
                ),
            });
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
        Ok(NymTopology::new(
            metadata.to_topology_metadata(),
            epoch_rewarded_set,
            Vec::new(),
        )
        .with_skimmed_nodes(&nodes))
    }

    async fn get_current_compatible_topology_inner(&mut self) -> Result<NymTopology, NymAPIError> {
        if self.config.use_extended_topology {
            self.try_get_extended_topology().await
        } else {
            // if we're not using extended topology, we're only getting active set mixnodes and gateways
            self.try_get_active_topology().await
        }
    }

    async fn get_current_compatible_topology(&mut self) -> Option<NymTopology> {
        let topology = match self.get_current_compatible_topology_inner().await {
            Ok(topology) => topology,
            Err(err) => {
                // if the error is due to the data inconsistency, it means we made the requests across epoch boundaries
                // and thus we can afford a single retry.
                // note that we have to throw away all the data we might have retrieved,
                // such as the rewarding set assignment (since it'd now correspond to the old epoch)
                if err.is_data_inconsistency() {
                    self.get_current_compatible_topology_inner()
                        .await
                        .inspect_err(|err| {
                            error!("failed to retrieve network topology after retry: {err}")
                        })
                        .ok()?
                } else {
                    error!("failed to retrieve network topology: {err}");
                    return None;
                }
            }
        };

        if !topology.is_minimally_routable() {
            error!("the current filtered active topology can't be used to construct any packets");
            return None;
        }

        Some(topology)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl TopologyProvider for NymApiTopologyProvider {
    async fn get_new_topology(&mut self) -> Option<NymTopology> {
        self.get_current_compatible_topology().await
    }
}

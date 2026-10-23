// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Where the LP view of the network comes from.
//!
//! The twin of [`nym_api_provider`](super::nym_api_provider), against the listing that carries LP
//! details. It describes a smaller network: a node that published none is absent here, which is the
//! point - an LP route can only name nodes it can actually reach.
//!
//! Only the all-nodes path is mirrored. The legacy provider also has an 'active' path that fetches
//! the mixing and entry sets separately; LP has no use for it yet, and leaving it out keeps the
//! amount of code that has to be deleted with legacy down.

use async_trait::async_trait;
use nym_mixnet_contract_common::EpochRewardedSet;
use nym_topology::lp::{LpTopology, LpTopologyProvider};
use nym_topology::provider_trait::ToTopologyMetadata;
use nym_validator_client::nym_api::NymApiClientExt;
use nym_validator_client::nym_api::error::NymAPIError;
use tracing::{debug, error};

use super::nym_api_provider::Config;

pub struct NymApiLpTopologyProvider {
    config: Config,
    validator_client: nym_http_api_client::Client,
}

impl NymApiLpTopologyProvider {
    pub fn new(config: impl Into<Config>, validator_client: nym_http_api_client::Client) -> Self {
        NymApiLpTopologyProvider {
            config: config.into(),
            validator_client,
        }
    }

    async fn try_get_topology(&mut self) -> Result<LpTopology, NymAPIError> {
        let rewarded_set_fut = self.validator_client.get_current_rewarded_set();
        let all_nodes_fut = self.validator_client.get_all_lp_nodes_with_metadata();

        let (rewarded_set, all_nodes_res) = futures::try_join!(rewarded_set_fut, all_nodes_fut)
            .inspect_err(|err| {
                error!("failed to get LP network nodes: {err}");
            })?;

        let metadata = all_nodes_res.metadata;
        let all_nodes = all_nodes_res.nodes;

        if rewarded_set.epoch_id != metadata.absolute_epoch_id {
            // the requests succeeded but returned data across epochs, so together they describe no
            // network that ever existed
            return Err(NymAPIError::InternalResponseInconsistency {
                url: self.validator_client.current_url().clone().into(),
                details: format!(
                    "retrieved rewarded set information for different epoch than the nodes details {} and {}",
                    rewarded_set.epoch_id, metadata.absolute_epoch_id
                ),
            });
        }

        debug!(
            "there are {} LP-capable nodes on the network (before filtering)",
            all_nodes.len()
        );

        let nodes_filtered = all_nodes
            .into_iter()
            .filter(|n| n.performance.round_to_integer() >= self.config.min_node_performance())
            .collect::<Vec<_>>();

        let epoch_rewarded_set: EpochRewardedSet = rewarded_set.into();

        // the rewarded set arrives describing the whole network; `LpTopology` narrows it to the
        // nodes present here, so a hop is only ever drawn from what an LP route can reach
        Ok(LpTopology::from_skimmed_nodes(
            metadata.to_topology_metadata(),
            epoch_rewarded_set,
            &nodes_filtered,
        ))
    }
}

#[async_trait]
impl LpTopologyProvider for NymApiLpTopologyProvider {
    async fn get_new_lp_topology(&mut self) -> Option<LpTopology> {
        match self.try_get_topology().await {
            Ok(topology) => Some(topology),
            Err(err) => {
                error!("failed to obtain the LP network topology: {err}");
                None
            }
        }
    }
}

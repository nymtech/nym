// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use log::{error, warn};
use nym_topology::provider_trait::TopologyProvider;
use nym_topology::{nym_topology_from_detailed, NymTopology, NymTopologyError};
use rand::prelude::SliceRandom;
use rand::thread_rng;
use url::Url;

pub struct NymApiTopologyProvider {
    validator_client: nym_validator_client::client::NymApiClient,
    nym_api_urls: Vec<Url>,

    client_version: String,
    currently_used_api: usize,
}

impl NymApiTopologyProvider {
    pub fn new(mut nym_api_urls: Vec<Url>, client_version: String) -> Self {
        nym_api_urls.shuffle(&mut thread_rng());

        NymApiTopologyProvider {
            validator_client: nym_validator_client::client::NymApiClient::new(
                nym_api_urls[0].clone(),
            ),
            nym_api_urls,
            client_version,
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

    /// Verifies whether nodes a reasonably distributed among all mix layers.
    ///
    /// In ideal world we would have 33% nodes on layer 1, 33% on layer 2 and 33% on layer 3.
    /// However, this is a rather unrealistic expectation, instead we check whether there exists
    /// a layer with more than 66% of nodes or with fewer than 15% and if so, we trigger a failure.
    ///
    /// # Arguments
    ///
    /// * `topology`: active topology constructed from validator api data
    fn check_layer_distribution(
        &self,
        active_topology: &NymTopology,
    ) -> Result<(), NymTopologyError> {
        let lower_threshold = 0.15;
        let upper_threshold = 0.66;
        active_topology.ensure_even_layer_distribution(lower_threshold, upper_threshold)
    }

    async fn get_current_compatible_topology(&mut self) -> Option<NymTopology> {
        let mixnodes = match self.validator_client.get_cached_active_mixnodes().await {
            Err(err) => {
                error!("failed to get network mixnodes - {err}");
                return None;
            }
            Ok(mixes) => mixes,
        };

        let gateways = match self.validator_client.get_cached_described_gateways().await {
            Err(err) => {
                error!("failed to get network gateways - {err}");
                return None;
            }
            Ok(gateways) => gateways,
        };

        let nodes_described = match self.validator_client.get_cached_described_nodes().await {
            Err(err) => {
                error!("failed to get described nodes - {err}");
                return None;
            }
            Ok(epoch) => epoch,
        };

        let topology = nym_topology_from_detailed(mixnodes, gateways, nodes_described.clone())
            .filter_system_version(&self.client_version);

        if let Err(err) = self.check_layer_distribution(&topology) {
            warn!("The current filtered active topology has extremely skewed layer distribution. It cannot be used: {err}");
            self.use_next_nym_api();
            let empty_topology = NymTopology::empty().with_described_nodes(nodes_described);
            Some(empty_topology)
        } else {
            Some(topology)
        }
    }
}

// hehe, wasm
#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl TopologyProvider for NymApiTopologyProvider {
    async fn get_new_topology(&mut self) -> Option<NymTopology> {
        self.get_current_compatible_topology().await
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
impl TopologyProvider for NymApiTopologyProvider {
    async fn get_new_topology(&mut self) -> Option<NymTopology> {
        self.get_current_compatible_topology().await
    }
}

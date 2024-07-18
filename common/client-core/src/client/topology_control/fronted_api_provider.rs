// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use log::{debug, error, warn};
use nym_topology::provider_trait::TopologyProvider;
use nym_topology::{NymTopology, NymTopologyError};
use rand::prelude::SliceRandom;
use rand::thread_rng;
use url::Url;

use super::nym_api_provider::Config;

pub(crate) struct FrontedApiTopologyProvider {
    config: Config,

    validator_client: nym_validator_client::client::NymApiClient,
    nym_api_urls: Vec<Url>,
    fronting_domains: Vec<Url>,
    shuffling: Vec<usize>,

    client_version: String,
    currently_used_api: usize,
}

impl FrontedApiTopologyProvider {
    pub(crate) fn new(
        config: Config,
        nym_api_urls: Vec<Url>,
        fronting_domains: Vec<Url>,
        client_version: String,
    ) -> Self {
        //SW for the PoC, we assume same lenght between fronting domains and api_urls
        let mut shuffling = (0..nym_api_urls.len()).collect::<Vec<_>>();
        shuffling.shuffle(&mut thread_rng());

        FrontedApiTopologyProvider {
            config,
            validator_client: nym_validator_client::client::NymApiClient::new_fronted(
                nym_api_urls[shuffling[0]].clone(),
                fronting_domains[shuffling[0]].clone(),
            ),
            nym_api_urls,
            fronting_domains,
            shuffling,
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
        self.validator_client.change_nym_api_with_fronting(
            self.nym_api_urls[self.shuffling[self.currently_used_api]].clone(),
            self.fronting_domains[self.shuffling[self.currently_used_api]].clone(),
        );
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
        let mixnodes = match self
            .validator_client
            .get_basic_mixnodes(Some(self.client_version.clone()))
            .await
        {
            Err(err) => {
                error!("failed to get network mixnodes - {err}");
                return None;
            }
            Ok(mixes) => mixes,
        };

        let gateways = match self
            .validator_client
            .get_basic_gateways(Some(self.client_version.clone()))
            .await
        {
            Err(err) => {
                error!("failed to get network gateways - {err}");
                return None;
            }
            Ok(gateways) => gateways,
        };

        debug!(
            "there are {} mixnodes and {} gateways in total (before performance filtering)",
            mixnodes.len(),
            gateways.len()
        );

        let topology = NymTopology::from_unordered(
            mixnodes.iter().filter(|m| {
                m.performance.round_to_integer() >= self.config.min_mixnode_performance
            }),
            gateways.iter().filter(|g| {
                g.performance.round_to_integer() >= self.config.min_gateway_performance
            }),
        );

        if let Err(err) = self.check_layer_distribution(&topology) {
            warn!("The current filtered active topology has extremely skewed layer distribution. It cannot be used: {err}");
            self.use_next_nym_api();
            None
        } else {
            Some(topology)
        }
    }
}

// hehe, wasm
#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl TopologyProvider for FrontedApiTopologyProvider {
    async fn get_new_topology(&mut self) -> Option<NymTopology> {
        self.get_current_compatible_topology().await
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
impl TopologyProvider for FrontedApiTopologyProvider {
    async fn get_new_topology(&mut self) -> Option<NymTopology> {
        self.get_current_compatible_topology().await
    }
}

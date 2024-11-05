// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use log::{debug, error, warn};
use nym_topology::provider_trait::TopologyProvider;
use nym_topology::{NymTopology, NymTopologyError};
use nym_validator_client::UserAgent;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use url::Url;

// the same values as our current (10.06.24) blacklist
pub const DEFAULT_MIN_MIXNODE_PERFORMANCE: u8 = 50;
pub const DEFAULT_MIN_GATEWAY_PERFORMANCE: u8 = 50;

#[derive(Debug)]
pub struct Config {
    pub min_mixnode_performance: u8,
    pub min_gateway_performance: u8,
}

impl Default for Config {
    fn default() -> Self {
        // old values that decided on blacklist membership
        Config {
            min_mixnode_performance: DEFAULT_MIN_MIXNODE_PERFORMANCE,
            min_gateway_performance: DEFAULT_MIN_GATEWAY_PERFORMANCE,
        }
    }
}

pub struct NymApiTopologyProvider {
    config: Config,

    validator_client: nym_validator_client::client::NymApiClient,
    nym_api_urls: Vec<Url>,

    client_version: String,
    currently_used_api: usize,
}

impl NymApiTopologyProvider {
    pub fn new(
        config: Config,
        mut nym_api_urls: Vec<Url>,
        client_version: String,
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
            config,
            validator_client,
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
        #[allow(deprecated)]
        let mixnodes = match self
            .validator_client
            // .get_all_basic_active_mixing_assigned_nodes(Some(self.client_version.clone()))
            .get_basic_mixnodes(Some(self.client_version.clone()))
            .await
        {
            Err(err) => {
                error!("failed to get network mixnodes - {err}");
                return None;
            }
            Ok(mixes) => mixes,
        };

        #[allow(deprecated)]
        let gateways = match self
            .validator_client
            // .get_all_basic_entry_assigned_nodes(Some(self.client_version.clone()))
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

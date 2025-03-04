// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Caching, piecewise API Topology Provider
//!

#![warn(missing_docs)]

use async_trait::async_trait;
use log::{debug, error, warn};
use nym_topology::{
    providers::piecewise::{Config, NymTopologyProvider, PiecewiseTopologyProvider},
    EpochRewardedSet, NymTopology, RoutingNode,
};
use nym_validator_client::UserAgent;
use rand::{prelude::SliceRandom, thread_rng};
use url::Url;

/// Topology Provider build around a cached piecewise provider that uses the Nym API to
/// fetch changes and node details.
#[derive(Clone)]
pub struct NymApiTopologyProvider {
    inner: NymTopologyProvider<NymApiPiecewiseProvider>,
}

impl NymApiTopologyProvider {
    /// Construct a new thread safe Cached topology provider using the Nym API
    pub fn new(
        user_agent: UserAgent,
        nym_api_urls: Vec<Url>,
        config: Config,
        initial_topology: Option<NymTopology>,
    ) -> Self {
        let manager = NymApiPiecewiseProvider::new(nym_api_urls, Some(user_agent));
        let inner = NymTopologyProvider::new(manager, config, initial_topology);

        Self { inner }
    }
}

impl AsRef<NymTopologyProvider<NymApiPiecewiseProvider>> for NymApiTopologyProvider {
    fn as_ref(&self) -> &NymTopologyProvider<NymApiPiecewiseProvider> {
        &self.inner
    }
}

#[derive(Clone)]
struct NymApiPiecewiseProvider {
    validator_client: nym_validator_client::client::NymApiClient,
    nym_api_urls: Vec<Url>,
    currently_used_api: usize,
}

impl NymApiPiecewiseProvider {
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
impl PiecewiseTopologyProvider for NymApiPiecewiseProvider {
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

    async fn get_descriptor_batch(&mut self, ids: &[u32]) -> Option<Vec<RoutingNode>> {
        // Does this need to return a hashmap of RoutingNodes? that is moderately inconvenient
        // especially when the nodes themselves contain their node_id unless we expect to directly
        // use the result of this fn for lookups where we would otherwise for example, have to
        // iterate over a whole vec to find a specific node_id.
        let descriptor_vec = self
            .validator_client
            .retrieve_basic_nodes_batch(ids)
            .await
            .inspect_err(|err| {
                self.use_next_nym_api();
                error!("failed to get current rewarded set: {err}");
            })
            .ok()?;

        let mut out = Vec::new();
        for node in descriptor_vec {
            if let Ok(routing_node) = RoutingNode::try_from(&node) {
                let _ = out.push(routing_node);
            }
        }
        Some(out)
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

// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mix_nodes::MIXNODES_CACHE_REFRESH_RATE;
use crate::state::ExplorerApiStateContext;
use mixnet_contract_common::MixNodeBond;
use reqwest::Url;
use std::future::Future;
use validator_client::ValidatorClientError;

pub(crate) struct MixNodesTasks {
    state: ExplorerApiStateContext,
    validator_api_client: validator_client::ApiClient,
}

impl MixNodesTasks {
    pub(crate) fn new(state: ExplorerApiStateContext, validator_api_endpoint: Url) -> Self {
        MixNodesTasks {
            state,
            validator_api_client: validator_client::ApiClient::new(validator_api_endpoint),
        }
    }

    // a helper to remove duplicate code when grabbing active/rewarded/all mixnodes
    async fn retrieve_mixnodes<'a, F, Fut>(&'a self, f: F) -> Vec<MixNodeBond>
    where
        F: FnOnce(&'a validator_client::ApiClient) -> Fut,
        Fut: Future<Output = Result<Vec<MixNodeBond>, ValidatorClientError>>,
    {
        let bonds = match f(&self.validator_api_client).await {
            Ok(result) => result,
            Err(e) => {
                error!("Unable to retrieve mixnode bonds: {:?}", e);
                vec![]
            }
        };

        info!("Fetched {} mixnode bonds", bonds.len());
        bonds
    }

    async fn retrieve_all_mixnodes(&self) -> Vec<MixNodeBond> {
        info!("About to retrieve all mixnode bonds...");
        self.retrieve_mixnodes(validator_client::ApiClient::get_cached_mixnodes)
            .await
    }

    async fn retrieve_rewarded_mixnodes(&self) -> Vec<MixNodeBond> {
        info!("About to retrieve rewarded mixnode bonds...");
        self.retrieve_mixnodes(validator_client::ApiClient::get_cached_rewarded_mixnodes)
            .await
    }

    async fn retrieve_active_mixnodes(&self) -> Vec<MixNodeBond> {
        info!("About to retrieve active mixnode bonds...");
        self.retrieve_mixnodes(validator_client::ApiClient::get_cached_active_mixnodes)
            .await
    }

    async fn update_mixnode_cache(&self) {
        let all_bonds = self.retrieve_all_mixnodes().await;
        let rewarded_nodes = self
            .retrieve_rewarded_mixnodes()
            .await
            .into_iter()
            .map(|bond| bond.mix_node.identity_key)
            .collect();
        let active_nodes = self
            .retrieve_active_mixnodes()
            .await
            .into_iter()
            .map(|bond| bond.mix_node.identity_key)
            .collect();
        self.state
            .inner
            .mix_nodes
            .update_cache(all_bonds, rewarded_nodes, active_nodes)
            .await;
    }

    pub(crate) fn start(self) {
        info!("Spawning mix nodes task runner...");
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(MIXNODES_CACHE_REFRESH_RATE);
            loop {
                // wait for the next interval tick
                interval_timer.tick().await;

                info!("Updating mix node cache...");
                self.update_mixnode_cache().await;
                info!("Done");
            }
        });
    }
}

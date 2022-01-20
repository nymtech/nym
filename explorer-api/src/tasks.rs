// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::future::Future;

use mixnet_contract_common::{GatewayBond, MixNodeBond};
use validator_client::nymd::error::NymdError;
use validator_client::nymd::{Paging, QueryNymdClient, ValidatorResponse};
use validator_client::ValidatorClientError;

use crate::client::new_nymd_client;
use crate::mix_nodes::CACHE_REFRESH_RATE;
use crate::state::ExplorerApiStateContext;

pub(crate) struct ExplorerApiTasks {
    state: ExplorerApiStateContext,
    validator_client: validator_client::Client<QueryNymdClient>,
}

impl ExplorerApiTasks {
    pub(crate) fn new(state: ExplorerApiStateContext) -> Self {
        ExplorerApiTasks {
            state,
            validator_client: new_nymd_client(),
        }
    }

    // a helper to remove duplicate code when grabbing active/rewarded/all mixnodes
    async fn retrieve_mixnodes<'a, F, Fut>(&'a self, f: F) -> Vec<MixNodeBond>
    where
        F: FnOnce(&'a validator_client::Client<QueryNymdClient>) -> Fut,
        Fut: Future<Output = Result<Vec<MixNodeBond>, ValidatorClientError>>,
    {
        let bonds = match f(&self.validator_client).await {
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
        self.retrieve_mixnodes(validator_client::Client::get_cached_mixnodes)
            .await
    }

    async fn retrieve_all_gateways(&self) -> Result<Vec<GatewayBond>, ValidatorClientError> {
        info!("About to retrieve all gateways...");
        self.validator_client.get_cached_gateways().await
    }

    async fn retrieve_all_validators(&self) -> Result<ValidatorResponse, NymdError> {
        info!("About to retrieve all validators...");
        let height = self
            .validator_client
            .nymd
            .get_current_block_height()
            .await?;
        let response: ValidatorResponse = self
            .validator_client
            .nymd
            .get_validators(height.value(), Paging::All)
            .await?;
        info!("Fetched {} validators", response.validators.len());
        Ok(response)
    }

    async fn retrieve_rewarded_mixnodes(&self) -> Vec<MixNodeBond> {
        info!("About to retrieve rewarded mixnode bonds...");
        self.retrieve_mixnodes(validator_client::Client::get_cached_rewarded_mixnodes)
            .await
    }

    async fn retrieve_active_mixnodes(&self) -> Vec<MixNodeBond> {
        info!("About to retrieve active mixnode bonds...");
        self.retrieve_mixnodes(validator_client::Client::get_cached_active_mixnodes)
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
            .mixnodes
            .update_cache(all_bonds, rewarded_nodes, active_nodes)
            .await;
    }

    async fn update_validators_cache(&self) {
        match self.retrieve_all_validators().await {
            Ok(response) => self.state.inner.validators.update_cache(response).await,
            Err(e) => {
                error!("Failed to get validators: {:?}", e)
            }
        }
    }

    async fn update_gateways_cache(&self) {
        match self.retrieve_all_gateways().await {
            Ok(response) => self.state.inner.gateways.update_cache(response).await,
            Err(e) => {
                error!("Failed to get gateways: {:?}", e)
            }
        }
    }

    pub(crate) fn start(self) {
        info!("Spawning mix nodes task runner...");
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(CACHE_REFRESH_RATE);
            loop {
                // wait for the next interval tick
                interval_timer.tick().await;

                info!("Updating validator cache...");
                self.update_validators_cache().await;
                info!("Done");

                info!("Updating gateway cache...");
                self.update_gateways_cache().await;
                info!("Done");

                info!("Updating mix node cache...");
                self.update_mixnode_cache().await;
                info!("Done");
            }
        });
    }
}

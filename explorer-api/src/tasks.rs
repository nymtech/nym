// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_mixnet_contract_common::GatewayBond;
use nym_task::TaskClient;
use nym_validator_client::models::MixNodeBondAnnotated;
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::{Paging, TendermintRpcClient, ValidatorResponse};
use nym_validator_client::{QueryHttpRpcValidatorClient, ValidatorClientError};
use std::future::Future;

use crate::mix_nodes::CACHE_REFRESH_RATE;
use crate::state::ExplorerApiStateContext;

pub(crate) struct ExplorerApiTasks {
    state: ExplorerApiStateContext,
    shutdown: TaskClient,
}

impl ExplorerApiTasks {
    pub(crate) fn new(state: ExplorerApiStateContext, shutdown: TaskClient) -> Self {
        ExplorerApiTasks { state, shutdown }
    }

    // a helper to remove duplicate code when grabbing active/rewarded/all mixnodes
    async fn retrieve_mixnodes<'a, F, Fut>(&'a self, f: F) -> Vec<MixNodeBondAnnotated>
    where
        F: FnOnce(&'a QueryHttpRpcValidatorClient) -> Fut,
        Fut: Future<Output = Result<Vec<MixNodeBondAnnotated>, ValidatorClientError>>,
    {
        let bonds = f(&self.state.inner.validator_client.0)
            .await
            .unwrap_or_else(|err| {
                error!("Unable to retrieve mixnode bonds: {err}");
                vec![]
            });

        info!("Fetched {} mixnode bonds", bonds.len());
        bonds
    }

    async fn retrieve_all_mixnodes(&self) -> Vec<MixNodeBondAnnotated> {
        info!("About to retrieve all mixnode bonds...");
        self.retrieve_mixnodes(
            nym_validator_client::Client::get_cached_mixnodes_detailed_unfiltered,
        )
        .await
    }

    async fn retrieve_all_gateways(&self) -> Result<Vec<GatewayBond>, ValidatorClientError> {
        info!("About to retrieve all gateways...");
        self.state
            .inner
            .validator_client
            .0
            .get_cached_gateways()
            .await
    }

    async fn retrieve_all_validators(&self) -> Result<ValidatorResponse, NyxdError> {
        info!("About to retrieve all validators...");
        let height = self
            .state
            .inner
            .validator_client
            .0
            .nyxd
            .get_current_block_height()
            .await?;
        let response: ValidatorResponse = self
            .state
            .inner
            .validator_client
            .0
            .nyxd
            .validators(height, Paging::All)
            .await?;
        info!("Fetched {} validators", response.validators.len());
        Ok(response)
    }

    async fn retrieve_rewarded_mixnodes(&self) -> Vec<MixNodeBondAnnotated> {
        info!("About to retrieve rewarded mixnode bonds...");
        self.retrieve_mixnodes(nym_validator_client::Client::get_cached_rewarded_mixnodes_detailed)
            .await
    }

    async fn retrieve_active_mixnodes(&self) -> Vec<MixNodeBondAnnotated> {
        info!("About to retrieve active mixnode bonds...");
        self.retrieve_mixnodes(nym_validator_client::Client::get_cached_active_mixnodes_detailed)
            .await
    }

    async fn update_mixnode_cache(&self) {
        let all_bonds = self.retrieve_all_mixnodes().await;
        let rewarded_nodes = self
            .retrieve_rewarded_mixnodes()
            .await
            .into_iter()
            .map(|bond| bond.mix_id())
            .collect();
        let active_nodes = self
            .retrieve_active_mixnodes()
            .await
            .into_iter()
            .map(|bond| bond.mix_id())
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
            Err(err) => {
                error!("Failed to get validators: {err}")
            }
        }
    }

    async fn update_gateways_cache(&self) {
        match self.retrieve_all_gateways().await {
            Ok(response) => self.state.inner.gateways.update_cache(response).await,
            Err(err) => {
                error!("Failed to get gateways: {err}")
            }
        }
    }

    pub(crate) fn start(mut self) {
        info!("Spawning mix nodes task runner...");
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(CACHE_REFRESH_RATE);
            while !self.shutdown.is_shutdown() {
                tokio::select! {
                    _ = interval_timer.tick() => {
                        info!("Updating validator cache...");
                        self.update_validators_cache().await;
                        info!("Done");

                        info!("Updating gateway cache...");
                        self.update_gateways_cache().await;
                        info!("Done");

                        info!("Updating mix node cache...");
                        self.update_mixnode_cache().await;
                    }
                    _ = self.shutdown.recv() => {
                        trace!("Listener: Received shutdown");
                    }
                }
            }
        });
    }
}

// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mix_nodes::CACHE_REFRESH_RATE;
use crate::state::ExplorerApiStateContext;
use nym_mixnet_contract_common::{GatewayBond, MixNodeBond, NymNodeDetails};
use nym_task::TaskClient;
use nym_validator_client::models::{
    GatewayBondAnnotated, MixNodeBondAnnotated, NymNodeDescription,
};
use nym_validator_client::nyxd::contract_traits::PagedMixnetQueryClient;
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::{Paging, TendermintRpcClient, ValidatorResponse};
use nym_validator_client::{QueryHttpRpcValidatorClient, ValidatorClientError};
use std::future::Future;
use tokio::time::MissedTickBehavior;

pub(crate) struct ExplorerApiTasks {
    state: ExplorerApiStateContext,
    shutdown: TaskClient,
}

// allow usage of deprecated methods here as we actually want to be explicitly querying for legacy data
#[allow(deprecated)]
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

    async fn retrieve_bonded_nymnodes(&self) -> Result<Vec<NymNodeDetails>, ValidatorClientError> {
        info!("About to retrieve all nymnode bonds...");
        self.state
            .inner
            .validator_client
            .0
            .get_all_cached_bonded_nym_nodes()
            .await
    }

    async fn retrieve_node_descriptions(
        &self,
    ) -> Result<Vec<NymNodeDescription>, ValidatorClientError> {
        info!("About to retrieve node descriptions...");
        self.state
            .inner
            .validator_client
            .0
            .get_all_cached_described_nodes()
            .await
    }

    async fn retrieve_all_mixnodes(&self) -> Vec<MixNodeBondAnnotated> {
        info!("About to retrieve all mixnode bonds...");
        self.retrieve_mixnodes(
            nym_validator_client::Client::get_cached_mixnodes_detailed_unfiltered,
        )
        .await
    }

    async fn retrieve_all_gateways(
        &self,
    ) -> Result<Vec<GatewayBondAnnotated>, ValidatorClientError> {
        info!("About to retrieve all gateways...");
        self.state
            .inner
            .validator_client
            .0
            .get_cached_gateways_detailed_unfiltered()
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

    async fn retrieve_legacy_gateway_bonds(&self) -> Vec<GatewayBond> {
        self.state
            .inner
            .validator_client
            .0
            .nyxd
            .get_all_gateways()
            .await
            .unwrap_or(vec![])
    }

    async fn retrieve_legacy_mixnode_bonds(&self) -> Vec<MixNodeBond> {
        self.state
            .inner
            .validator_client
            .0
            .nyxd
            .get_all_mixnode_bonds()
            .await
            .unwrap_or(vec![])
    }

    async fn update_mixnode_cache(&self) {
        let legacy_mixnode_bonds = self.retrieve_legacy_mixnode_bonds().await;
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
            .update_cache(
                all_bonds,
                rewarded_nodes,
                active_nodes,
                legacy_mixnode_bonds,
            )
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
        let legacy_gateway_bonds = self.retrieve_legacy_gateway_bonds().await;
        match self.retrieve_all_gateways().await {
            Ok(response) => {
                self.state
                    .inner
                    .gateways
                    .update_cache(response, legacy_gateway_bonds)
                    .await
            }
            Err(err) => {
                error!("Failed to get gateways: {err}")
            }
        }
    }

    async fn update_nymnodes_cache(&self) {
        let nym_node_bonds = self.retrieve_bonded_nymnodes().await.unwrap_or_else(|err| {
            error!("failed to retrieve nym node bonds: {err}");
            Vec::new()
        });

        let all_descriptions = self
            .retrieve_node_descriptions()
            .await
            .unwrap_or_else(|err| {
                error!("failed to retrieve node descriptions: {err}");
                Vec::new()
            });

        self.state
            .inner
            .nymnodes
            .update_cache(nym_node_bonds, all_descriptions)
            .await
    }

    pub(crate) fn start(mut self) {
        info!("Spawning mix nodes task runner...");
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(CACHE_REFRESH_RATE);
            interval_timer.set_missed_tick_behavior(MissedTickBehavior::Skip);

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

                        info!("Updating nymnode cache...");
                        self.update_nymnodes_cache().await;
                        info!("Done");
                    }
                    _ = self.shutdown.recv() => {
                        trace!("Listener: Received shutdown");
                    }
                }
            }
        });
    }
}

// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet_contract_cache::cache::data::{ConfigScoreData, MixnetContractCacheData};
use crate::nyxd::Client;
use crate::support::caching::refresher::CacheItemProvider;
use anyhow::Result;
use async_trait::async_trait;
use nym_api_requests::models::LegacyGatewayBondWithId;
use nym_validator_client::nyxd::error::NyxdError;
use std::collections::HashMap;
use tracing::info;

pub struct MixnetContractDataProvider {
    nyxd_client: Client,
}

#[async_trait]
impl CacheItemProvider for MixnetContractDataProvider {
    type Item = MixnetContractCacheData;
    type Error = NyxdError;

    async fn try_refresh(&mut self) -> std::result::Result<Option<Self::Item>, Self::Error> {
        self.refresh().await.map(Some)
    }
}

impl MixnetContractDataProvider {
    pub(crate) fn new(nyxd_client: Client) -> Self {
        MixnetContractDataProvider { nyxd_client }
    }

    async fn refresh(&self) -> Result<MixnetContractCacheData, NyxdError> {
        let current_reward_params = self.nyxd_client.get_current_rewarding_parameters().await?;
        let current_interval = self.nyxd_client.get_current_interval().await?.interval;
        let contract_state = self.nyxd_client.get_mixnet_contract_state().await?;

        let nym_nodes = self.nyxd_client.get_nymnodes().await?;
        let legacy_mixnode_details = self.nyxd_client.get_mixnodes().await?;
        let legacy_gateway_bonds = self.nyxd_client.get_gateways().await?;
        let legacy_gateway_ids: HashMap<_, _> = self
            .nyxd_client
            .get_gateway_ids()
            .await?
            .into_iter()
            .map(|id| (id.identity, id.node_id))
            .collect();

        let mut legacy_gateways = Vec::with_capacity(legacy_gateway_bonds.len());
        #[allow(clippy::panic)]
        for bond in legacy_gateway_bonds {
            // we explicitly panic here because that value MUST exist.
            // if it doesn't, we messed up the migration and we have big problems
            let node_id = *legacy_gateway_ids.get(bond.identity()).unwrap_or_else(|| {
                panic!(
                    "CONTRACT DATA INCONSISTENCY: MISSING GATEWAY ID FOR: {}",
                    bond.identity()
                )
            });
            legacy_gateways.push(LegacyGatewayBondWithId { bond, node_id })
        }

        let rewarded_set = self.nyxd_client.get_rewarded_set_nodes().await?;
        let key_rotation_state = self.nyxd_client.get_key_rotation_state().await?;
        let config_score_params = self.nyxd_client.get_config_score_params().await?;
        let nym_node_version_history = self.nyxd_client.get_nym_node_version_history().await?;

        info!(
            "Updating validator cache. There are {} [legacy] mixnodes, {} [legacy] gateways and {} nym nodes",
            legacy_mixnode_details.len(),
            legacy_gateways.len(),
            nym_nodes.len(),
        );

        Ok(MixnetContractCacheData {
            rewarding_denom: contract_state.rewarding_denom,
            legacy_mixnodes: legacy_mixnode_details,
            legacy_gateways,
            nym_nodes,
            rewarded_set: rewarded_set.into(),
            config_score_data: ConfigScoreData {
                config_score_params,
                nym_node_version_history,
            },
            current_reward_params,
            current_interval,
            key_rotation_state,
        })
    }
}

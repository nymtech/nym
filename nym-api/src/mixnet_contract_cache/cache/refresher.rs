// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet_contract_cache::cache::data::{ConfigScoreData, MixnetContractCacheData};
use crate::nyxd::Client;
use crate::support::caching::refresher::CacheItemProvider;
use anyhow::Result;
use async_trait::async_trait;
use nym_api_requests::legacy::{
    LegacyGatewayBondWithId, LegacyMixNodeBondWithLayer, LegacyMixNodeDetailsWithLayer,
};
use nym_mixnet_contract_common::LegacyMixLayer;
use nym_validator_client::nyxd::error::NyxdError;
use rand::prelude::SliceRandom;
use rand::rngs::OsRng;
use std::collections::HashMap;
use std::collections::HashSet;
use tracing::info;

pub struct MixnetContractDataProvider {
    nyxd_client: Client,
}

#[async_trait]
impl CacheItemProvider for MixnetContractDataProvider {
    type Item = MixnetContractCacheData;
    type Error = NyxdError;

    async fn try_refresh(&self) -> std::result::Result<Self::Item, Self::Error> {
        self.refresh().await
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
        let mixnode_details = self.nyxd_client.get_mixnodes().await?;
        let gateway_bonds = self.nyxd_client.get_gateways().await?;
        let gateway_ids: HashMap<_, _> = self
            .nyxd_client
            .get_gateway_ids()
            .await?
            .into_iter()
            .map(|id| (id.identity, id.node_id))
            .collect();

        let mut legacy_gateways = Vec::with_capacity(gateway_bonds.len());
        #[allow(clippy::panic)]
        for bond in gateway_bonds {
            // we explicitly panic here because that value MUST exist.
            // if it doesn't, we messed up the migration and we have big problems
            let node_id = *gateway_ids.get(bond.identity()).unwrap_or_else(|| {
                panic!(
                    "CONTRACT DATA INCONSISTENCY: MISSING GATEWAY ID FOR: {}",
                    bond.identity()
                )
            });
            legacy_gateways.push(LegacyGatewayBondWithId { bond, node_id })
        }

        let rewarded_set = self.nyxd_client.get_rewarded_set_nodes().await?;
        let layer1 = rewarded_set
            .assignment
            .layer1
            .iter()
            .collect::<HashSet<_>>();
        let layer2 = rewarded_set
            .assignment
            .layer2
            .iter()
            .collect::<HashSet<_>>();
        let layer3 = rewarded_set
            .assignment
            .layer3
            .iter()
            .collect::<HashSet<_>>();

        let layer_choices = [
            LegacyMixLayer::One,
            LegacyMixLayer::Two,
            LegacyMixLayer::Three,
        ];
        let mut rng = OsRng;
        let mut legacy_mixnodes = Vec::with_capacity(mixnode_details.len());
        for detail in mixnode_details {
            // if node is not in the rewarded set, well.
            // slap a random layer on it because legacy clients don't understand a concept of layerless mixnodes
            let layer = if layer1.contains(&detail.mix_id()) {
                LegacyMixLayer::One
            } else if layer2.contains(&detail.mix_id()) {
                LegacyMixLayer::Two
            } else if layer3.contains(&detail.mix_id()) {
                LegacyMixLayer::Three
            } else {
                // SAFETY: the slice is not empty so the unwrap is fine
                #[allow(clippy::unwrap_used)]
                layer_choices.choose(&mut rng).copied().unwrap()
            };

            legacy_mixnodes.push(LegacyMixNodeDetailsWithLayer {
                bond_information: LegacyMixNodeBondWithLayer {
                    bond: detail.bond_information,
                    layer,
                },
                rewarding_details: detail.rewarding_details,
                pending_changes: detail.pending_changes.into(),
            })
        }

        let key_rotation_state = self.nyxd_client.get_key_rotation_state().await?;
        let config_score_params = self.nyxd_client.get_config_score_params().await?;
        let nym_node_version_history = self.nyxd_client.get_nym_node_version_history().await?;

        info!(
            "Updating validator cache. There are {} [legacy] mixnodes, {} [legacy] gateways and {} nym nodes",
            legacy_mixnodes.len(),
            legacy_gateways.len(),
            nym_nodes.len(),
        );

        Ok(MixnetContractCacheData {
            rewarding_denom: contract_state.rewarding_denom,
            legacy_mixnodes,
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

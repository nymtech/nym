// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_api_requests::legacy::{LegacyGatewayBondWithId, LegacyMixNodeDetailsWithLayer};
use nym_api_requests::models::ConfigScoreDataResponse;
use nym_mixnet_contract_common::{
    ConfigScoreParams, HistoricalNymNodeVersionEntry, Interval, KeyRotationState, NymNodeDetails,
    RewardingParams,
};
use nym_topology::CachedEpochRewardedSet;

#[derive(Clone)]
pub(crate) struct ConfigScoreData {
    pub(crate) config_score_params: ConfigScoreParams,
    pub(crate) nym_node_version_history: Vec<HistoricalNymNodeVersionEntry>,
}

impl From<ConfigScoreData> for ConfigScoreDataResponse {
    fn from(value: ConfigScoreData) -> Self {
        ConfigScoreDataResponse {
            parameters: value.config_score_params.into(),
            version_history: value
                .nym_node_version_history
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

pub(crate) struct MixnetContractCacheData {
    pub(crate) rewarding_denom: String,

    pub(crate) legacy_mixnodes: Vec<LegacyMixNodeDetailsWithLayer>,
    pub(crate) legacy_gateways: Vec<LegacyGatewayBondWithId>,
    pub(crate) nym_nodes: Vec<NymNodeDetails>,
    pub(crate) rewarded_set: CachedEpochRewardedSet,

    pub(crate) config_score_data: ConfigScoreData,
    pub(crate) current_reward_params: RewardingParams,
    pub(crate) current_interval: Interval,
    pub(crate) key_rotation_state: KeyRotationState,
}

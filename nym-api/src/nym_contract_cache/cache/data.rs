// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::caching::Cache;
use nym_api_requests::legacy::{LegacyGatewayBondWithId, LegacyMixNodeDetailsWithLayer};
use nym_api_requests::models::ConfigScoreDataResponse;
use nym_contracts_common::ContractBuildInformation;
use nym_mixnet_contract_common::{
    ConfigScoreParams, HistoricalNymNodeVersionEntry, Interval, KeyRotationState, NodeId,
    NymNodeDetails, RewardingParams,
};
use nym_topology::CachedEpochRewardedSet;
use nym_validator_client::nyxd::AccountId;
use std::collections::{HashMap, HashSet};

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

pub(crate) struct ContractCacheData {
    pub(crate) legacy_mixnodes: Cache<Vec<LegacyMixNodeDetailsWithLayer>>,
    pub(crate) legacy_gateways: Cache<Vec<LegacyGatewayBondWithId>>,
    pub(crate) nym_nodes: Cache<Vec<NymNodeDetails>>,
    pub(crate) rewarded_set: Cache<CachedEpochRewardedSet>,

    // this purposely does not deal with nym-nodes as they don't have a concept of a blacklist.
    // instead clients are meant to be filtering out them themselves based on the provided scores.
    pub(crate) legacy_mixnodes_blacklist: Cache<HashSet<NodeId>>,
    pub(crate) legacy_gateways_blacklist: Cache<HashSet<NodeId>>,

    pub(crate) config_score_data: Cache<Option<ConfigScoreData>>,
    pub(crate) current_reward_params: Cache<Option<RewardingParams>>,
    pub(crate) current_interval: Cache<Option<Interval>>,
    pub(crate) key_rotation_state: Cache<Option<KeyRotationState>>,

    pub(crate) contracts_info: Cache<CachedContractsInfo>,
}

impl ContractCacheData {
    pub(crate) fn new() -> Self {
        ContractCacheData {
            legacy_mixnodes: Cache::default(),
            legacy_gateways: Cache::default(),
            nym_nodes: Default::default(),
            rewarded_set: Cache::default(),

            legacy_mixnodes_blacklist: Cache::default(),
            legacy_gateways_blacklist: Cache::default(),
            current_interval: Cache::default(),
            current_reward_params: Cache::default(),
            contracts_info: Cache::default(),
            config_score_data: Default::default(),
            key_rotation_state: Default::default(),
        }
    }
}

type ContractAddress = String;
pub type CachedContractsInfo = HashMap<ContractAddress, CachedContractInfo>;

#[derive(Clone)]
pub struct CachedContractInfo {
    pub(crate) address: Option<AccountId>,
    pub(crate) base: Option<cw2::ContractVersion>,
    pub(crate) detailed: Option<ContractBuildInformation>,
}

impl CachedContractInfo {
    pub fn new(
        address: Option<&AccountId>,
        base: Option<cw2::ContractVersion>,
        detailed: Option<ContractBuildInformation>,
    ) -> Self {
        Self {
            address: address.cloned(),
            base,
            detailed,
        }
    }
}

// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::caching::Cache;
use nym_api_requests::legacy::{LegacyGatewayBondWithId, LegacyMixNodeDetailsWithLayer};
use nym_contracts_common::ContractBuildInformation;
use nym_mixnet_contract_common::{Interval, NodeId, NymNodeDetails, RewardedSet, RewardingParams};
use nym_validator_client::nyxd::AccountId;
use std::collections::{HashMap, HashSet};

pub(crate) struct ValidatorCacheData {
    pub(crate) legacy_mixnodes: Cache<Vec<LegacyMixNodeDetailsWithLayer>>,
    pub(crate) legacy_gateways: Cache<Vec<LegacyGatewayBondWithId>>,
    pub(crate) nym_nodes: Cache<Vec<NymNodeDetails>>,
    pub(crate) rewarded_set: Cache<RewardedSet>,

    // this purposely does not deal with nym-nodes as they don't have a concept of a blacklist.
    // instead clients are meant to be filtering out them themselves based on the provided scores.
    pub(crate) legacy_mixnodes_blacklist: Cache<HashSet<NodeId>>,
    pub(crate) legacy_gateways_blacklist: Cache<HashSet<NodeId>>,

    pub(crate) current_reward_params: Cache<Option<RewardingParams>>,
    pub(crate) current_interval: Cache<Option<Interval>>,

    pub(crate) contracts_info: Cache<CachedContractsInfo>,
}

impl ValidatorCacheData {
    pub(crate) fn new() -> Self {
        ValidatorCacheData {
            legacy_mixnodes: Cache::default(),
            legacy_gateways: Cache::default(),
            nym_nodes: Default::default(),
            rewarded_set: Cache::default(),

            legacy_mixnodes_blacklist: Cache::default(),
            legacy_gateways_blacklist: Cache::default(),
            current_interval: Cache::default(),
            current_reward_params: Cache::default(),
            contracts_info: Cache::default(),
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

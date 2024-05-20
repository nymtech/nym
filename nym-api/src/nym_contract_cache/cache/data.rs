// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::caching::Cache;
use nym_contracts_common::ContractBuildInformation;
use nym_mixnet_contract_common::{
    families::FamilyHead, GatewayBond, IdentityKey, Interval, MixId, MixNodeDetails,
    RewardingParams,
};
use nym_validator_client::nyxd::AccountId;
use std::collections::{HashMap, HashSet};

pub(crate) struct ValidatorCacheData {
    pub(crate) mixnodes: Cache<Vec<MixNodeDetails>>,
    pub(crate) gateways: Cache<Vec<GatewayBond>>,

    pub(crate) mixnodes_blacklist: Cache<HashSet<MixId>>,
    pub(crate) gateways_blacklist: Cache<HashSet<IdentityKey>>,

    pub(crate) rewarded_set: Cache<Vec<MixNodeDetails>>,
    pub(crate) active_set: Cache<Vec<MixNodeDetails>>,

    pub(crate) current_reward_params: Cache<Option<RewardingParams>>,
    pub(crate) current_interval: Cache<Option<Interval>>,

    pub(crate) mix_to_family: Cache<Vec<(IdentityKey, FamilyHead)>>,

    pub(crate) contracts_info: Cache<CachedContractsInfo>,
}

impl ValidatorCacheData {
    pub(crate) fn new() -> Self {
        ValidatorCacheData {
            mixnodes: Cache::default(),
            gateways: Cache::default(),
            rewarded_set: Cache::default(),
            active_set: Cache::default(),
            mixnodes_blacklist: Cache::default(),
            gateways_blacklist: Cache::default(),
            current_interval: Cache::default(),
            current_reward_params: Cache::default(),
            mix_to_family: Cache::default(),
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

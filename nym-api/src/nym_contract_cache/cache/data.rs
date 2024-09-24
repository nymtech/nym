// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::caching::Cache;
use nym_api_requests::legacy::{LegacyGatewayBondWithId, LegacyMixNodeDetailsWithLayer};
use nym_contracts_common::ContractBuildInformation;
use nym_mixnet_contract_common::nym_node::Role;
use nym_mixnet_contract_common::{Interval, NodeId, NymNodeDetails, RewardedSet, RewardingParams};
use nym_validator_client::nyxd::AccountId;
use std::collections::{HashMap, HashSet};

#[derive(Default, Clone)]
pub(crate) struct CachedRewardedSet {
    pub(crate) entry_gateways: HashSet<NodeId>,

    pub(crate) exit_gateways: HashSet<NodeId>,

    pub(crate) layer1: HashSet<NodeId>,

    pub(crate) layer2: HashSet<NodeId>,

    pub(crate) layer3: HashSet<NodeId>,

    pub(crate) standby: HashSet<NodeId>,
}

impl From<RewardedSet> for CachedRewardedSet {
    fn from(value: RewardedSet) -> Self {
        CachedRewardedSet {
            entry_gateways: value.entry_gateways.into_iter().collect(),
            exit_gateways: value.exit_gateways.into_iter().collect(),
            layer1: value.layer1.into_iter().collect(),
            layer2: value.layer2.into_iter().collect(),
            layer3: value.layer3.into_iter().collect(),
            standby: value.standby.into_iter().collect(),
        }
    }
}

impl From<CachedRewardedSet> for RewardedSet {
    fn from(value: CachedRewardedSet) -> Self {
        RewardedSet {
            entry_gateways: value.entry_gateways.into_iter().collect(),
            exit_gateways: value.exit_gateways.into_iter().collect(),
            layer1: value.layer1.into_iter().collect(),
            layer2: value.layer2.into_iter().collect(),
            layer3: value.layer3.into_iter().collect(),
            standby: value.standby.into_iter().collect(),
        }
    }
}

impl CachedRewardedSet {
    pub(crate) fn role(&self, node_id: NodeId) -> Option<Role> {
        if self.entry_gateways.contains(&node_id) {
            Some(Role::EntryGateway)
        } else if self.exit_gateways.contains(&node_id) {
            Some(Role::ExitGateway)
        } else if self.layer1.contains(&node_id) {
            Some(Role::Layer1)
        } else if self.layer2.contains(&node_id) {
            Some(Role::Layer2)
        } else if self.layer3.contains(&node_id) {
            Some(Role::Layer3)
        } else if self.standby.contains(&node_id) {
            Some(Role::Standby)
        } else {
            None
        }
    }

    pub fn try_get_mix_layer(&self, node_id: &NodeId) -> Option<u8> {
        if self.layer1.contains(node_id) {
            Some(1)
        } else if self.layer2.contains(node_id) {
            Some(2)
        } else if self.layer3.contains(node_id) {
            Some(3)
        } else {
            None
        }
    }

    pub fn is_standby(&self, node_id: &NodeId) -> bool {
        self.standby.contains(node_id)
    }

    pub fn is_active_mixnode(&self, node_id: &NodeId) -> bool {
        self.layer1.contains(node_id)
            || self.layer2.contains(node_id)
            || self.layer3.contains(node_id)
    }

    #[allow(dead_code)]
    pub(crate) fn gateways(&self) -> HashSet<NodeId> {
        let mut gateways =
            HashSet::with_capacity(self.entry_gateways.len() + self.exit_gateways.len());
        gateways.extend(&self.entry_gateways);
        gateways.extend(&self.exit_gateways);
        gateways
    }

    pub(crate) fn active_mixnodes(&self) -> HashSet<NodeId> {
        let mut mixnodes =
            HashSet::with_capacity(self.layer1.len() + self.layer2.len() + self.layer3.len());
        mixnodes.extend(&self.layer1);
        mixnodes.extend(&self.layer2);
        mixnodes.extend(&self.layer3);
        mixnodes
    }
}

pub(crate) struct ValidatorCacheData {
    pub(crate) legacy_mixnodes: Cache<Vec<LegacyMixNodeDetailsWithLayer>>,
    pub(crate) legacy_gateways: Cache<Vec<LegacyGatewayBondWithId>>,
    pub(crate) nym_nodes: Cache<Vec<NymNodeDetails>>,
    pub(crate) rewarded_set: Cache<CachedRewardedSet>,

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

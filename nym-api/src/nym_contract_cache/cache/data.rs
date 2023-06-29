// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::support::caching::Cache;
use nym_mixnet_contract_common::{
    families::FamilyHead, GatewayBond, IdentityKey, Interval, MixId, MixNodeDetails,
    RewardingParams,
};
use nym_name_service_common::NameEntry;
use nym_service_provider_directory_common::ServiceInfo;
use std::collections::HashSet;

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

    pub(crate) service_providers: Cache<Vec<ServiceInfo>>,
    pub(crate) registered_names: Cache<Vec<NameEntry>>,
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
            service_providers: Cache::default(),
            registered_names: Cache::default(),
        }
    }
}

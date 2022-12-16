use std::collections::HashSet;

use mixnet_contract_common::{
    families::FamilyHead, GatewayBond, IdentityKey, Interval, MixId, MixNodeDetails,
    RewardingParams,
};

use crate::support::caching::Cache;

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
        }
    }
}

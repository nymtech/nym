// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnode::MixNodeCostParams;
use crate::reward_params::NodeRewardParams;
use crate::ContractStateParams;
use crate::{Gateway, IdentityKey, MixNode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub rewarding_validator_address: String,
    pub rewarding_denom: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    BondMixnode {
        mix_node: MixNode,
        cost_params: MixNodeCostParams,
        owner_signature: String,
    },
    UnbondMixnode {},
    UnbondMixnodeOnBehalf {
        owner: String,
    },
    UpdateRewardingValidatorAddress {
        address: String,
    },
    UpdateContractStateParams {
        updated_parameters: ContractStateParams,
    },

    // un-re-implemented as of yet:
    InitEpoch {},
    ReconcileDelegations {},
    CheckpointMixnodes {},
    CompoundOperatorRewardOnBehalf {
        owner: String,
    },
    CompoundDelegatorRewardOnBehalf {
        owner: String,
        mix_identity: IdentityKey,
    },
    CompoundOperatorReward {},
    CompoundDelegatorReward {
        mix_identity: IdentityKey,
    },

    UpdateMixnodeConfig {
        profit_margin_percent: u8,
    },
    UpdateMixnodeConfigOnBehalf {
        profit_margin_percent: u8,
        owner: String,
    },
    BondGateway {
        gateway: Gateway,
        owner_signature: String,
    },
    UnbondGateway {},

    DelegateToMixnode {
        mix_identity: IdentityKey,
    },

    UndelegateFromMixnode {
        mix_identity: IdentityKey,
    },

    RewardMixnode {
        identity: IdentityKey,
        // percentage value in range 0-100
        params: NodeRewardParams,
    },
    // RewardNextMixDelegators {
    //     mix_identity: IdentityKey,
    //     // id of the current rewarding interval
    //     interval_id: u32,
    // },
    DelegateToMixnodeOnBehalf {
        mix_identity: IdentityKey,
        delegate: String,
    },
    UndelegateFromMixnodeOnBehalf {
        mix_identity: IdentityKey,
        delegate: String,
    },
    BondMixnodeOnBehalf {
        mix_node: MixNode,
        owner: String,
        owner_signature: String,
    },
    BondGatewayOnBehalf {
        gateway: Gateway,
        owner: String,
        owner_signature: String,
    },
    UnbondGatewayOnBehalf {
        owner: String,
    },
    WriteRewardedSet {
        rewarded_set: Vec<IdentityKey>,
        expected_active_set_size: u32,
    },
    // AdvanceCurrentInterval {},
    AdvanceCurrentEpoch {},
    ClaimOperatorReward {},
    ClaimOperatorRewardOnBehalf {
        owner: String,
    },
    ClaimDelegatorReward {
        mix_identity: IdentityKey,
    },
    ClaimDelegatorRewardOnBehalf {
        mix_identity: IdentityKey,
        owner: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetBlacklistedNodes {},
    GetCurrentOperatorCost {},
    GetRewardingValidatorAddress {},
    GetAllDelegationKeys {},
    DebugGetAllDelegationValues {},
    GetContractVersion {},
    GetMixNodes {
        limit: Option<u32>,
        start_after: Option<IdentityKey>,
    },
    GetGateways {
        start_after: Option<IdentityKey>,
        limit: Option<u32>,
    },
    OwnsMixnode {
        address: String,
    },
    OwnsGateway {
        address: String,
    },
    GetMixnodeBond {
        identity: IdentityKey,
    },
    GetGatewayBond {
        identity: IdentityKey,
    },
    StateParams {},
    // gets all [paged] delegations associated with particular mixnode
    GetMixnodeDelegations {
        mix_identity: IdentityKey,
        // since `start_after` is user-provided input, we can't use `Addr` as we
        // can't guarantee it's validated.
        start_after: Option<(String, u64)>,
        limit: Option<u32>,
    },
    // gets all [paged] delegations associated with particular delegator
    GetDelegatorDelegations {
        // since `delegator` is user-provided input, we can't use `Addr` as we
        // can't guarantee it's validated.
        delegator: String,
        start_after: Option<IdentityKey>,
        limit: Option<u32>,
    },
    // gets delegation associated with particular mixnode, delegator pair
    GetDelegationDetails {
        mix_identity: IdentityKey,
        delegator: String,
        proxy: Option<String>,
    },
    LayerDistribution {},
    GetRewardPool {},
    GetCirculatingSupply {},
    GetStakingSupply {},
    GetIntervalRewardPercent {},
    GetSybilResistancePercent {},
    GetActiveSetWorkFactor {},
    GetRewardingStatus {
        mix_identity: IdentityKey,
        interval_id: u32,
    },
    GetRewardedSet {
        height: Option<u64>,
        start_after: Option<IdentityKey>,
        limit: Option<u32>,
    },
    GetRewardedSetUpdateDetails {},
    GetCurrentRewardedSetHeight {},
    GetRewardedSetRefreshBlocks {},
    GetCurrentEpoch {},
    GetEpochsInInterval {},
    QueryOperatorReward {
        address: String,
    },
    QueryDelegatorReward {
        address: String,
        mix_identity: IdentityKey,
        proxy: Option<String>,
    },
    GetPendingDelegationEvents {
        owner_address: String,
        proxy_address: Option<String>,
    },
    GetCheckpointsForMixnode {
        mix_identity: IdentityKey,
    },
    GetMixnodeAtHeight {
        mix_identity: IdentityKey,
        height: u64,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {
    pub mixnet_denom: String,
    nodes_to_remove: Option<Vec<NodeToRemove>>,
}

impl MigrateMsg {
    pub fn nodes_to_remove(&self) -> Vec<NodeToRemove> {
        self.nodes_to_remove.clone().unwrap_or_default()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct NodeToRemove {
    owner: String,
    proxy: Option<String>,
}

impl NodeToRemove {
    pub fn owner(&self) -> &str {
        &self.owner
    }

    pub fn proxy(&self) -> Option<&String> {
        self.proxy.as_ref()
    }
}

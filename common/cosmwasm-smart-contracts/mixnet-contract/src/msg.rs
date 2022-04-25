// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::reward_params::NodeRewardParams;
use crate::ContractStateParams;
use crate::{Gateway, IdentityKey, MixNode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub rewarding_validator_address: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateRewardingValidatorAddress {
        address: String,
    },
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
    BondMixnode {
        mix_node: MixNode,
        owner_signature: String,
    },
    UnbondMixnode {},
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
    UpdateContractStateParams(ContractStateParams),

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
    UnbondMixnodeOnBehalf {
        owner: String,
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
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
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
    },
    GetPendingDelegationEvents {
        owner_address: String,
        proxy_address: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

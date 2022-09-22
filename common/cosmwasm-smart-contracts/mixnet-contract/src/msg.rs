// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::reward_params::NodeRewardParams;
use crate::{ContractStateParams, Layer, SphinxKey};
use crate::{Gateway, IdentityKey, MixNode};
use cosmwasm_std::{Addr, Coin};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    pub rewarding_validator_address: String,
    pub mixnet_denom: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
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
    CompoundReward {
        operator: Option<String>,
        delegator: Option<String>,
        mix_identity: Option<IdentityKey>,
        proxy: Option<String>,
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetAllDelegationValuesPaged {
        start_after: Option<(IdentityKey, Vec<u8>, u64)>,
        limit: Option<u32>,
    },
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

// all of those `serde rename` are here to reduce the rpc response size to bare minimum

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[serde(rename = "op")]
pub enum V2MigrationOperation {
    #[serde(rename = "m1")]
    MigrateOperator {
        #[serde(rename = "a1")]
        node_identity: String,
    },

    #[serde(rename = "m2")]
    MigrateDelegator {
        #[serde(rename = "a1")]
        address: Addr,

        #[serde(rename = "a2")]
        node_identity: String,

        #[serde(rename = "a3")]
        proxy: Option<Addr>,

        #[serde(rename = "a4")]
        new_mix_id: Option<u32>,
    },

    #[serde(rename = "m3")]
    RemoveOperator {
        #[serde(rename = "a1")]
        node_identity: String,
    },

    #[serde(rename = "m4")]
    RemoveDelegator {
        #[serde(rename = "a1")]
        address: Addr,

        #[serde(rename = "a2")]
        node_identity: String,

        #[serde(rename = "a3")]
        proxy: Option<Addr>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SpecialV2ExecuteMsg {
    #[serde(rename = "m1")]
    SaveOperator {
        #[serde(rename = "a1")]
        host: String,

        #[serde(rename = "a2")]
        mix_port: u16,

        #[serde(rename = "a3")]
        verloc_port: u16,

        #[serde(rename = "a4")]
        http_api_port: u16,

        #[serde(rename = "a5")]
        sphinx_key: SphinxKey,

        #[serde(rename = "a6")]
        identity_key: IdentityKey,

        #[serde(rename = "a7")]
        version: String,

        #[serde(rename = "a8")]
        pledge_amount: Coin,

        #[serde(rename = "a9")]
        owner: Addr,

        #[serde(rename = "a10")]
        block_height: u64,

        #[serde(rename = "a11")]
        profit_margin_percent: u8,

        #[serde(rename = "a12")]
        proxy: Option<Addr>,
    },
    #[serde(rename = "m2")]
    SaveDelegation {
        #[serde(rename = "a1")]
        owner: Addr,

        #[serde(rename = "a2")]
        mix_id: u32,

        #[serde(rename = "a3")]
        amount: Coin,

        #[serde(rename = "a4")]
        block_height: u64,

        #[serde(rename = "a5")]
        proxy: Option<Addr>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {
    pub v2_contract_address: String,
    pub vesting_contract_address: String,
    pub operations: Vec<V2MigrationOperation>,
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

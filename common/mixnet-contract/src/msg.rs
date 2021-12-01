// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnode::NodeRewardParams;
use crate::ContractSettingsParams;
use crate::{Gateway, IdentityKey, MixNode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    BondMixnode {
        mix_node: MixNode,
        owner_signature: String,
    },
    UnbondMixnode {},
    BondGateway {
        gateway: Gateway,
        owner_signature: String,
    },
    UnbondGateway {},
    UpdateContractSettings(ContractSettingsParams),

    DelegateToMixnode {
        mix_identity: IdentityKey,
    },

    UndelegateFromMixnode {
        mix_identity: IdentityKey,
    },

    BeginMixnodeRewarding {
        // nonce of the current rewarding interval
        rewarding_interval_nonce: u32,
    },

    FinishMixnodeRewarding {
        // nonce of the current rewarding interval
        rewarding_interval_nonce: u32,
    },

    RewardMixnodeV2 {
        identity: IdentityKey,
        // percentage value in range 0-100
        params: NodeRewardParams,

        // nonce of the current rewarding interval
        rewarding_interval_nonce: u32,
    },
    RewardNextMixDelegators {
        mix_identity: IdentityKey,
        // nonce of the current rewarding interval
        rewarding_interval_nonce: u32,
    },
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
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
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
    CurrentRewardingInterval {},
    // gets all [paged] delegations in the entire network
    // TODO: do we even want that?
    GetAllNetworkDelegations {
        start_after: Option<(IdentityKey, String)>,
        limit: Option<u32>,
    },
    // gets all [paged] delegations associated with particular mixnode
    GetMixnodeDelegations {
        mix_identity: IdentityKey,
        // since `start_after` is user-provided input, we can't use `Addr` as we
        // can't guarantee it's validated.
        start_after: Option<String>,
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
    },
    LayerDistribution {},
    GetRewardPool {},
    GetCirculatingSupply {},
    GetEpochRewardPercent {},
    GetSybilResistancePercent {},
    GetRewardingStatus {
        mix_identity: IdentityKey,
        rewarding_interval_nonce: u32,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

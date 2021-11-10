// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnode::NodeRewardParams;
use crate::StateParams;
use crate::{Gateway, IdentityKey, MixNode};
use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    BondMixnode {
        mix_node: MixNode,
    },
    UnbondMixnode {},
    BondGateway {
        gateway: Gateway,
    },
    UnbondGateway {},
    UpdateStateParams(StateParams),

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

    RewardMixnode {
        identity: IdentityKey,
        // percentage value in range 0-100
        uptime: u32,

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
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetMixNodes {
        limit: Option<u32>,
        start_after: Option<IdentityKey>,
    },
    GetGateways {
        start_after: Option<IdentityKey>,
        limit: Option<u32>,
    },
    OwnsMixnode {
        address: Addr,
    },
    OwnsGateway {
        address: Addr,
    },
    StateParams {},
    CurrentRewardingInterval {},
    GetMixDelegations {
        mix_identity: IdentityKey,
        start_after: Option<Addr>,
        limit: Option<u32>,
    },
    GetAllMixDelegations {
        start_after: Option<Vec<u8>>,
        limit: Option<u32>,
    },
    GetReverseMixDelegations {
        delegation_owner: Addr,
        start_after: Option<IdentityKey>,
        limit: Option<u32>,
    },
    GetMixDelegation {
        mix_identity: IdentityKey,
        address: Addr,
    },
    LayerDistribution {},
    GetRewardPool {},
    GetCirculatingSupply {},
    GetEpochRewardPercent {},
    GetSybilResistancePercent {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

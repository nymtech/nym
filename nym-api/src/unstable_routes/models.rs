// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use cosmwasm_std::{Addr, Coin};
use nym_topology::NodeId;
use serde::{Deserialize, Serialize};
use utoipa::schema;

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema, utoipa::ToResponse)]
#[schema(title = "Coin")]
pub struct CoinSchema {
    pub denom: String,
    pub amount: u128,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema, utoipa::ToResponse)]
pub struct NyxAccountDelegationDetails {
    pub node_id: NodeId,
    #[schema(value_type = CoinSchema)]
    pub delegated: Coin,
    pub height: u64,
    #[schema(value_type = Option<String>)]
    pub proxy: Option<Addr>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema, utoipa::ToResponse)]
pub struct NyxAccountDelegationRewardDetails {
    pub node_id: NodeId,
    #[schema(value_type = CoinSchema)]
    pub rewards: Coin,
    #[schema(value_type = String)]
    pub amount_staked: Coin,
    pub node_still_fully_bonded: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema, utoipa::ToResponse)]
pub struct NymVestingAccount {
    #[schema(value_type = CoinSchema)]
    pub locked: Coin,
    #[schema(value_type = CoinSchema)]
    pub vested: Coin,
    #[schema(value_type = CoinSchema)]
    pub vesting: Coin,
    #[schema(value_type = CoinSchema)]
    pub spendable: Coin,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema, utoipa::ToResponse)]
pub struct NyxAccountDetails {
    pub address: String,
    #[schema(value_type = CoinSchema)]
    pub balance: Coin,
    #[schema(value_type = CoinSchema)]
    pub total_value: Coin,
    pub delegations: Vec<NyxAccountDelegationDetails>,
    pub accumulated_rewards: Vec<NyxAccountDelegationRewardDetails>,
    #[schema(value_type = String)]
    pub total_delegations: Coin,
    #[schema(value_type = CoinSchema)]
    pub claimable_rewards: Coin,
    pub vesting_account: Option<NymVestingAccount>,
    #[schema(value_type = Option<CoinSchema>)]
    pub operator_rewards: Option<Coin>,
}

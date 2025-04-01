// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use cosmwasm_std::{Addr, Coin};
use nym_topology::NodeId;
use serde::{Deserialize, Serialize};
use utoipa::schema;

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema, utoipa::ToResponse)]
pub struct NyxAccountDelegationDetails {
    pub node_id: NodeId,
    #[schema(value_type = String)]
    pub delegated: Coin,
    pub height: u64,
    #[schema(value_type = String)]
    pub proxy: Option<Addr>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema, utoipa::ToResponse)]
pub struct NyxAccountDelegationRewardDetails {
    pub node_id: NodeId,
    #[schema(value_type = String)]
    pub rewards: Coin,
    #[schema(value_type = String)]
    pub amount_staked: Coin,
    pub node_still_fully_bonded: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema, utoipa::ToResponse)]
pub struct NymVestingAccount {
    #[schema(value_type = String)]
    pub locked: Coin,
    #[schema(value_type = String)]
    pub vested: Coin,
    #[schema(value_type = String)]
    pub vesting: Coin,
    #[schema(value_type = String)]
    pub spendable: Coin,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema, utoipa::ToResponse)]
pub struct NyxAccountDetails {
    pub address: String,
    #[schema(value_type = Vec<String>)]
    pub balances: Vec<Coin>,
    #[schema(value_type = String)]
    pub total_value: Coin,
    pub delegations: Vec<NyxAccountDelegationDetails>,
    pub accumulated_rewards: Vec<NyxAccountDelegationRewardDetails>,
    #[schema(value_type = String)]
    pub total_delegations: Coin,
    #[schema(value_type = String)]
    pub claimable_rewards: Coin,
    pub vesting_account: Option<NymVestingAccount>,
    #[schema(value_type = Option<String>)]
    pub operator_rewards: Option<Coin>,
}

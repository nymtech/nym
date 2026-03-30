// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::cw_serde;
use cosmwasm_std::Coin;

#[cw_serde]
pub struct PoolCounters {
    /// Represents the total amount of funds deposited into the contract.
    pub total_deposited: Coin,

    /// Represents the total amount of funds redeemed from the contract that got transferred into the holding account.
    pub total_redeemed: Coin,

    /// Represents the total amount of tickets requested to be redeemed from the contract and get moved into the holding account,
    /// after that functionality got disabled.
    #[serde(default)]
    pub tickets_requested_and_not_redeemed: u64,
}

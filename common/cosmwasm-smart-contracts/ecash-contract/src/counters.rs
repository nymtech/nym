// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::cw_serde;
use cosmwasm_std::Coin;

#[cw_serde]
pub struct PoolCounters {
    pub total_deposited: Coin,
    pub total_redeemed_gateways: Coin,
    pub total_redeemed_holding: Coin,
}

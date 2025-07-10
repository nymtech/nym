// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::CoinSchema;
use cosmwasm_std::Coin;
use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

#[derive(Clone, Serialize, Deserialize, schemars::JsonSchema, ToSchema, ToResponse)]
pub struct CirculatingSupplyResponse {
    #[schema(value_type = CoinSchema)]
    pub total_supply: Coin,
    #[schema(value_type = CoinSchema)]
    pub mixmining_reserve: Coin,
    #[schema(value_type = CoinSchema)]
    pub vesting_tokens: Coin,
    #[schema(value_type = CoinSchema)]
    pub circulating_supply: Coin,
}

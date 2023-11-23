// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::caching::Cache;
use nym_api_requests::models::CirculatingSupplyResponse;
use nym_validator_client::nyxd::Coin;

pub(crate) struct CirculatingSupplyCacheData {
    // no need to cache that one as it's constant, but let's put it here for consistency sake
    pub(crate) total_supply: Coin,
    pub(crate) mixmining_reserve: Cache<Coin>,
    pub(crate) vesting_tokens: Cache<Coin>,
    pub(crate) circulating_supply: Cache<Coin>,
}

impl CirculatingSupplyCacheData {
    pub fn new(mix_denom: String) -> CirculatingSupplyCacheData {
        let zero_coin = Coin::new(0, &mix_denom);

        CirculatingSupplyCacheData {
            total_supply: Coin::new(1_000_000_000_000_000, mix_denom),
            mixmining_reserve: Cache::new(zero_coin.clone()),
            vesting_tokens: Cache::new(zero_coin.clone()),
            circulating_supply: Cache::new(zero_coin),
        }
    }
}

impl<'a> From<&'a CirculatingSupplyCacheData> for CirculatingSupplyResponse {
    fn from(value: &'a CirculatingSupplyCacheData) -> Self {
        CirculatingSupplyResponse {
            total_supply: value.total_supply.clone().into(),
            mixmining_reserve: value.mixmining_reserve.clone().into(),
            vesting_tokens: value.vesting_tokens.clone().into(),
            circulating_supply: value.circulating_supply.clone().into(),
        }
    }
}

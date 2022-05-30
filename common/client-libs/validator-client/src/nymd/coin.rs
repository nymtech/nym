// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
use std::fmt;

pub use cosmrs::Coin as CosmosCoin;
pub use cosmwasm_std::Coin as CosmWasmCoin;

// the reason the coin is created here as opposed to different place in the codebase is that
// eventually we want to either publish the cosmwasm client separately or commit it to
// some other project, like cosmrs. Either way, in that case we can't really have
// a dependency on an internal type
pub struct Coin {
    pub amount: u128,
    pub denom: String,
}

impl fmt::Display for Coin {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.amount, self.denom)
    }
}

impl From<Coin> for CosmosCoin {
    fn from(coin: Coin) -> Self {
        assert!(
            coin.amount <= u64::MAX as u128,
            "the coin amount is higher than the maximum supported by cosmrs"
        );

        CosmosCoin {
            denom: coin
                .denom
                .parse()
                .expect("the coin should have had a valid denom!"),
            amount: (coin.amount as u64).into(),
        }
    }
}

impl From<Coin> for CosmWasmCoin {
    fn from(coin: Coin) -> Self {
        CosmWasmCoin::new(coin.amount, coin.denom)
    }
}

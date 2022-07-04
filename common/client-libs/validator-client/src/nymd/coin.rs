// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
use serde::{Deserialize, Serialize};
use std::fmt;

pub use cosmrs::Coin as CosmosCoin;
pub use cosmwasm_std::Coin as CosmWasmCoin;

#[derive(Serialize, Deserialize, Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct MismatchedDenoms;

// the reason the coin is created here as opposed to different place in the codebase is that
// eventually we want to either publish the cosmwasm client separately or commit it to
// some other project, like cosmrs. Either way, in that case we can't really have
// a dependency on an internal type
#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct Coin {
    pub amount: u128,
    pub denom: String,
}

impl Coin {
    pub fn new<S: Into<String>>(amount: u128, denom: S) -> Self {
        Coin {
            amount,
            denom: denom.into(),
        }
    }

    pub fn try_add(&self, other: &Self) -> Result<Self, MismatchedDenoms> {
        if self.denom != other.denom {
            Err(MismatchedDenoms)
        } else {
            Ok(Coin {
                amount: self.amount + other.amount,
                denom: self.denom.clone(),
            })
        }
    }
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

impl From<CosmosCoin> for Coin {
    fn from(coin: CosmosCoin) -> Self {
        Coin {
            amount: coin
                .amount
                .to_string()
                .parse()
                .expect("somehow failed to parse string representation of u64"),
            denom: coin.denom.to_string(),
        }
    }
}

impl From<Coin> for CosmWasmCoin {
    fn from(coin: Coin) -> Self {
        CosmWasmCoin::new(coin.amount, coin.denom)
    }
}

impl From<CosmWasmCoin> for Coin {
    fn from(coin: CosmWasmCoin) -> Self {
        Coin {
            amount: coin.amount.u128(),
            denom: coin.denom,
        }
    }
}

pub trait CoinConverter {
    type Target;

    fn convert_coin(&self) -> Self::Target;
}

impl CoinConverter for CosmosCoin {
    type Target = CosmWasmCoin;

    fn convert_coin(&self) -> Self::Target {
        CosmWasmCoin::new(
            self.amount
                .to_string()
                .parse()
                .expect("cosmos coin had an invalid amount assigned"),
            self.denom.to_string(),
        )
    }
}

impl CoinConverter for CosmWasmCoin {
    type Target = CosmosCoin;

    fn convert_coin(&self) -> Self::Target {
        assert!(
            self.amount.u128() <= u64::MAX as u128,
            "the coin amount is higher than the maximum supported by cosmrs"
        );

        CosmosCoin {
            denom: self
                .denom
                .parse()
                .expect("cosmwasm coin had an invalid amount assigned"),
            amount: (self.amount.u128() as u64).into(),
        }
    }
}

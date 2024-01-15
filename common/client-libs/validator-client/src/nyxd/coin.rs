// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::{Gas, GasPrice};
pub use cosmrs::Coin as CosmosCoin;
pub use cosmwasm_std::Coin as CosmWasmCoin;
use cosmwasm_std::{Fraction, Uint128};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Div;
use std::str::FromStr;
use thiserror::Error;

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

impl Div<GasPrice> for Coin {
    type Output = Gas;

    fn div(self, rhs: GasPrice) -> Self::Output {
        &self / rhs
    }
}

impl<'a> Div<GasPrice> for &'a Coin {
    type Output = Gas;

    fn div(self, rhs: GasPrice) -> Self::Output {
        if self.denom != rhs.denom {
            panic!(
                "attempted to use two different denoms for gas calculation ({} and {})",
                self.denom, rhs.denom
            );
        }

        // tsk, tsk. somebody tried to divide by zero here!
        let Some(gas_price_inv) = rhs.amount.inv() else {
            panic!("attempted to divide by zero!")
        };

        let implicit_gas_limit = gas_price_inv * Uint128::new(self.amount);
        if implicit_gas_limit.u128() >= u64::MAX as u128 {
            u64::MAX
        } else {
            implicit_gas_limit.u128() as u64
        }
    }
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

// unfortunately cosmwasm didn't re-export this correct so we just redefine its
#[derive(Error, Debug, PartialEq, Eq)]
pub enum CoinFromStrError {
    #[error("Missing denominator")]
    MissingDenom,
    #[error("Missing amount or non-digit characters in amount")]
    MissingAmount,
    #[error("Invalid amount: {0}")]
    InvalidAmount(#[from] std::num::ParseIntError),
}

impl FromStr for Coin {
    type Err = CoinFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let pos = s
            .find(|c: char| !c.is_ascii_digit())
            .ok_or(CoinFromStrError::MissingDenom)?;
        let (amount, denom) = s.split_at(pos);

        if amount.is_empty() {
            return Err(CoinFromStrError::MissingAmount);
        }

        Ok(Coin {
            amount: amount.parse::<u128>()?,
            denom: denom.to_string(),
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn division_by_zero_gas_price() {
        let gas_price: GasPrice = "0unym".parse().unwrap();
        let amount = Coin::new(123, "unym");
        let _res = amount / gas_price;
    }

    #[test]
    #[should_panic]
    fn division_by_gas_price_of_different_denom() {
        let gas_price: GasPrice = "0.025unyx".parse().unwrap();
        let amount = Coin::new(123, "unym");
        let _res = amount / gas_price;
    }

    #[test]
    fn gas_price_division() {
        let amount = Coin::new(3938, "unym");
        let gas_price = "0.025unym".parse().unwrap();
        let res = amount / gas_price;
        assert_eq!(157520, res);

        let amount = Coin::new(1234567890, "unym");
        let gas_price = "0.025unym".parse().unwrap();
        let res = amount / gas_price;
        assert_eq!(49382715600, res);

        let amount = Coin::new(1, "unym");
        let gas_price = "0.025unym".parse().unwrap();
        let res = amount / gas_price;
        assert_eq!(40, res);

        let amount = Coin::new(150_000_000, "unym");
        let gas_price = "0.001234unym".parse().unwrap();
        let res = amount / gas_price;
        assert_eq!(121555915721, res);

        let amount = Coin::new(150_000_000, "unym");
        let gas_price = "1unym".parse().unwrap();
        let res = amount / gas_price;
        assert_eq!(150_000_000, res);

        let amount = Coin::new(150_000_000, "unym");
        let gas_price = "1234.56unym".parse().unwrap();
        let res = amount / gas_price;
        assert_eq!(121500, res);
    }

    #[test]
    fn gas_price_division_identity() {
        let amount = Coin::new(1234567890, "unym");
        let gas_price: GasPrice = "0.025unym".parse().unwrap();
        let res1 = (&amount) / gas_price.clone();
        let res2 = &gas_price * res1;

        assert_eq!(amount, Coin::from(res2));
    }
}

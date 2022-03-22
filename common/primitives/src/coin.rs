// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::PrimitivesError;
use core::fmt;
use cosmwasm_std::Uint128;
use std::str::FromStr;

/// Common Coin type for the backend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Coin {
    pub amount: Uint128,
    pub denom: String,
}

impl Coin {
    pub fn new(amount: u128, denom: impl Into<String>) -> Coin {
        Coin {
            amount: Uint128::new(amount),
            denom: denom.into(),
        }
    }
}

impl fmt::Display for Coin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.amount, self.denom)
    }
}

impl TryFrom<cosmrs::Coin> for Coin {
    type Error = PrimitivesError;

    fn try_from(cosmos_coin: cosmrs::Coin) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: cosmos_coin.amount.to_string().as_str().try_into()?,
            denom: cosmos_coin.denom.to_string(),
        })
    }
}

impl TryFrom<Coin> for cosmrs::Coin {
    type Error = PrimitivesError;

    fn try_from(coin: Coin) -> Result<Self, Self::Error> {
        Ok(Self {
            denom: cosmrs::Denom::from_str(&coin.denom)?,
            amount: cosmrs::Decimal::from_str(&coin.amount.to_string())?,
        })
    }
}

impl From<cosmwasm_std::Coin> for Coin {
    fn from(cosmwasm_coin: cosmwasm_std::Coin) -> Self {
        Self {
            amount: cosmwasm_coin.amount,
            denom: cosmwasm_coin.denom,
        }
    }
}

impl From<Coin> for cosmwasm_std::Coin {
    fn from(coin: Coin) -> Self {
        Self {
            amount: coin.amount,
            denom: coin.denom,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_to_and_from_cosmwasm_coin() {
        let coin = Coin::new(42, "ucoin");
        let cosmwasm_coin: cosmwasm_std::Coin = coin.clone().into();
        assert_eq!(coin, Coin::from(cosmwasm_coin));
    }

    #[test]
    fn convert_to_and_from_cosmos_coin() {
        let coin = Coin::new(42, "ucoin");
        let cosmos_coin: cosmrs::Coin = coin.clone().try_into().unwrap();
        assert_eq!(coin, Coin::try_from(cosmos_coin).unwrap());
    }
}

// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd::error::NymdError;
use config::defaults;
use cosmrs::tx::Gas;
use cosmrs::{Coin, Denom};
use cosmwasm_std::{Decimal, Fraction, Uint128};
use std::ops::Mul;
use std::str::FromStr;

/// A gas price, i.e. the price of a single unit of gas. This is typically a fraction of
/// the smallest fee token unit, such as 0.012utoken.
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct GasPrice {
    // I really hate the combination of cosmwasm Decimal with cosmos-sdk Denom,
    // but cosmos-sdk's Decimal is too basic for our needs
    pub amount: Decimal,

    pub denom: Denom,
}

impl<'a> Mul<Gas> for &'a GasPrice {
    type Output = Coin;

    fn mul(self, gas_limit: Gas) -> Self::Output {
        let limit_uint128 = Uint128::from(gas_limit.value());
        let mut amount = self.amount * limit_uint128;

        let gas_price_numerator = self.amount.numerator();
        let gas_price_denominator = self.amount.denominator();

        // gas price is a fraction of the smallest fee token unit, so we must ensure that
        // for any multiplication, we have rounded up
        //
        // I don't really like the this solution as it has a theoretical chance of
        // overflowing (internally cosmwasm uses U256 to avoid that)
        // however, realistically that is impossible to happen as the resultant value
        // would have to be way higher than our token limit of 10^15 (1 billion of tokens * 1 million for denomination)
        // and max value of u128 is approximately 10^38
        if limit_uint128 * gas_price_numerator > amount * gas_price_denominator {
            amount += Uint128::new(1);
        }

        assert!(amount.u128() <= u64::MAX as u128);
        Coin {
            denom: self.denom.clone(),
            amount: (amount.u128() as u64).into(),
        }
    }
}

impl FromStr for GasPrice {
    type Err = NymdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let possible_amount = s
            .chars()
            .take_while(|c| c.is_ascii_digit() || c == &'.')
            .collect::<String>();
        let amount_len = possible_amount.len();
        let amount = possible_amount
            .parse()
            .map_err(|_| NymdError::MalformedGasPrice)?;
        let possible_denom = s.chars().skip(amount_len).collect::<String>();
        let denom = possible_denom
            .parse()
            .map_err(|_| NymdError::MalformedGasPrice)?;

        Ok(GasPrice { amount, denom })
    }
}

impl GasPrice {
    pub fn new_with_default_price(denom: String) -> Result<Self, NymdError> {
        format!("{}{}", defaults::GAS_PRICE_AMOUNT, denom).parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_gas_price_is_valid() {
        let denom = "unym".parse().unwrap();
        let _ = GasPrice::default(denom);
    }

    #[test]
    fn gas_price_parsing() {
        assert_eq!(
            GasPrice {
                amount: "0.025".parse().unwrap(),
                denom: "upunk".parse().unwrap()
            },
            "0.025upunk".parse().unwrap()
        );

        assert_eq!(
            GasPrice {
                amount: "123".parse().unwrap(),
                denom: "upunk".parse().unwrap()
            },
            "123upunk".parse().unwrap()
        );

        assert!(".25upunk".parse::<GasPrice>().is_err());
        assert!("0.025 upunk".parse::<GasPrice>().is_err());
        assert!("0.025UPUNK".parse::<GasPrice>().is_err());
    }

    #[test]
    fn gas_limit_multiplication() {
        // real world example that caused an issue when the result was rounded down
        let gas_price: GasPrice = "0.025upunk".parse().unwrap();
        let gas_limit: Gas = 157500u64.into();

        let fee = &gas_price * gas_limit;
        // the failing behaviour was result value of 3937
        assert_eq!(fee.amount, 3938u64.into());
    }
}

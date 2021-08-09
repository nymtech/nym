// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ValidatorClientError;
use config::defaults;
use cosmos_sdk::Denom;
use cosmwasm_std::Decimal;
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

impl FromStr for GasPrice {
    type Err = ValidatorClientError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // s.split()
        let possible_amount = s
            .chars()
            .take_while(|c| c.is_ascii_digit() || c == &'.')
            .collect::<String>();
        let amount_len = possible_amount.len();
        let amount = possible_amount
            .parse()
            .map_err(|_| ValidatorClientError::MalformedGasPrice)?;
        let possible_denom = s.chars().skip(amount_len).collect::<String>();
        let denom = possible_denom
            .parse()
            .map_err(|_| ValidatorClientError::MalformedGasPrice)?;

        Ok(GasPrice { amount, denom })
    }
}

impl Default for GasPrice {
    fn default() -> Self {
        format!("{}{}", defaults::GAS_PRICE_AMOUNT, defaults::DENOM)
            .parse()
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

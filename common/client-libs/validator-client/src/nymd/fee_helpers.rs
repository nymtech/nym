// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd::GasPrice;
use cosmrs::tx::{Fee, Gas};
use cosmrs::Coin;
use cosmwasm_std::Uint128;
use serde::{Deserialize, Serialize};
use std::fmt;
use ts_rs::TS;

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, TS)]
pub enum Operation {
    Upload,
    Init,
    Migrate,
    ChangeAdmin,
    Send,

    BondMixnode,
    UnbondMixnode,
    DelegateToMixnode,
    UndelegateFromMixnode,

    BondGateway,
    UnbondGateway,
    DelegateToGateway,
    UndelegateFromGateway,

    UpdateStateParams,
}

pub(crate) fn calculate_fee(gas_price: &GasPrice, gas_limit: Gas) -> Coin {
    let limit_uint128 = Uint128::from(gas_limit.value());
    let amount = gas_price.amount * limit_uint128;
    assert!(amount.u128() <= u64::MAX as u128);
    Coin {
        denom: gas_price.denom.clone(),
        amount: (amount.u128() as u64).into(),
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Operation::Upload => f.write_str("Upload"),
            Operation::Init => f.write_str("Init"),
            Operation::Migrate => f.write_str("Migrate"),
            Operation::ChangeAdmin => f.write_str("ChangeAdmin"),
            Operation::Send => f.write_str("Send"),
            Operation::BondMixnode => f.write_str("BondMixnode"),
            Operation::UnbondMixnode => f.write_str("UnbondMixnode"),
            Operation::DelegateToMixnode => f.write_str("DelegateToMixnode"),
            Operation::UndelegateFromMixnode => f.write_str("UndelegateFromMixnode"),
            Operation::BondGateway => f.write_str("BondGateway"),
            Operation::UnbondGateway => f.write_str("UnbondGateway"),
            Operation::DelegateToGateway => f.write_str("DelegateToGateway"),
            Operation::UndelegateFromGateway => f.write_str("UndelegateFromGateway"),
            Operation::UpdateStateParams => f.write_str("UpdateStateParams"),
        }
    }
}

impl Operation {
    // TODO: some value tweaking
    pub fn default_gas_limit(&self) -> Gas {
        match self {
            Operation::Upload => 2_500_000u64.into(),
            Operation::Init => 500_000u64.into(),
            Operation::Migrate => 200_000u64.into(),
            Operation::ChangeAdmin => 80_000u64.into(),
            Operation::Send => 80_000u64.into(),

            Operation::BondMixnode => 175_000u64.into(),
            Operation::UnbondMixnode => 175_000u64.into(),
            Operation::DelegateToMixnode => 175_000u64.into(),
            Operation::UndelegateFromMixnode => 175_000u64.into(),

            Operation::BondGateway => 175_000u64.into(),
            Operation::UnbondGateway => 175_000u64.into(),
            Operation::DelegateToGateway => 175_000u64.into(),
            Operation::UndelegateFromGateway => 175_000u64.into(),

            Operation::UpdateStateParams => 175_000u64.into(),
        }
    }

    pub(crate) fn determine_fee(&self, gas_price: &GasPrice, gas_limit: Option<Gas>) -> Fee {
        // we need to know 2 of the following 3 parameters (the third one is being implicit) in order to construct Fee:
        // (source: https://docs.cosmos.network/v0.42/basics/gas-fees.html)
        // - gas price
        // - gas limit
        // - fees
        let gas_limit = gas_limit.unwrap_or_else(|| self.default_gas_limit());
        let fee = calculate_fee(gas_price, gas_limit);
        Fee::from_amount_and_gas(fee, gas_limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculating_fee() {
        let expected = Coin {
            denom: "upunk".parse().unwrap(),
            amount: 1000u64.into(),
        };
        let gas_price = "1upunk".parse().unwrap();
        let gas_limit = 1000u64.into();

        assert_eq!(expected, calculate_fee(&gas_price, gas_limit));

        let expected = Coin {
            denom: "upunk".parse().unwrap(),
            amount: 50u64.into(),
        };
        let gas_price = "0.05upunk".parse().unwrap();
        let gas_limit = 1000u64.into();

        assert_eq!(expected, calculate_fee(&gas_price, gas_limit));

        let expected = Coin {
            denom: "upunk".parse().unwrap(),
            amount: 100000u64.into(),
        };
        let gas_price = "100upunk".parse().unwrap();
        let gas_limit = 1000u64.into();

        assert_eq!(expected, calculate_fee(&gas_price, gas_limit))
    }
}

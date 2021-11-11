// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd::GasPrice;
use cosmrs::tx::{Fee, Gas};
use cosmrs::Coin;
use serde::{Deserialize, Serialize};
use std::fmt;

#[cfg_attr(feature = "ts-rs", derive(ts_rs::TS))]
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
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

    UpdateStateParams,

    BeginMixnodeRewarding,
    FinishMixnodeRewarding,
}

pub(crate) fn calculate_fee(gas_price: &GasPrice, gas_limit: Gas) -> Coin {
    gas_price * gas_limit
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
            Operation::BondGateway => f.write_str("BondGateway"),
            Operation::UnbondGateway => f.write_str("UnbondGateway"),
            Operation::DelegateToMixnode => f.write_str("DelegateToMixnode"),
            Operation::UndelegateFromMixnode => f.write_str("UndelegateFromMixnode"),
            Operation::UpdateStateParams => f.write_str("UpdateStateParams"),
            Operation::BeginMixnodeRewarding => f.write_str("BeginMixnodeRewarding"),
            Operation::FinishMixnodeRewarding => f.write_str("FinishMixnodeRewarding"),
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

            Operation::UpdateStateParams => 175_000u64.into(),
            Operation::BeginMixnodeRewarding => 175_000u64.into(),
            Operation::FinishMixnodeRewarding => 175_000u64.into(),
        }
    }

    pub(crate) fn determine_custom_fee(gas_price: &GasPrice, gas_limit: Gas) -> Fee {
        // we need to know 2 of the following 3 parameters (the third one is being implicit) in order to construct Fee:
        // (source: https://docs.cosmos.network/v0.42/basics/gas-fees.html)
        // - gas price
        // - gas limit
        // - fees
        let fee = calculate_fee(gas_price, gas_limit);
        Fee::from_amount_and_gas(fee, gas_limit)
    }

    pub(crate) fn determine_fee(&self, gas_price: &GasPrice, gas_limit: Option<Gas>) -> Fee {
        let gas_limit = gas_limit.unwrap_or_else(|| self.default_gas_limit());
        Self::determine_custom_fee(gas_price, gas_limit)
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

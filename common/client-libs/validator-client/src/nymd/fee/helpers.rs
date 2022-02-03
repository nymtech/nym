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
    BondMixnodeOnBehalf,
    UnbondMixnode,
    UnbondMixnodeOnBehalf,
    UpdateMixnodeConfig,
    DelegateToMixnode,
    DelegateToMixnodeOnBehalf,
    UndelegateFromMixnode,
    UndelegateFromMixnodeOnBehalf,

    BondGateway,
    BondGatewayOnBehalf,
    UnbondGateway,
    UnbondGatewayOnBehalf,

    UpdateContractSettings,

    BeginMixnodeRewarding,
    FinishMixnodeRewarding,

    TrackUnbondGateway,
    TrackUnbondMixnode,
    WithdrawVestedCoins,
    TrackUndelegation,
    CreatePeriodicVestingAccount,

    AdvanceCurrentInterval,
    WriteRewardedSet,
    ClearRewardedSet,
    UpdateMixnetAddress,
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
            Operation::BondMixnodeOnBehalf => f.write_str("BondMixnodeOnBehalf"),
            Operation::UnbondMixnode => f.write_str("UnbondMixnode"),
            Operation::UpdateMixnodeConfig => f.write_str("UpdateMixnodeConfig"),
            Operation::UnbondMixnodeOnBehalf => f.write_str("UnbondMixnodeOnBehalf"),
            Operation::BondGateway => f.write_str("BondGateway"),
            Operation::BondGatewayOnBehalf => f.write_str("BondGatewayOnBehalf"),
            Operation::UnbondGateway => f.write_str("UnbondGateway"),
            Operation::UnbondGatewayOnBehalf => f.write_str("UnbondGatewayOnBehalf"),
            Operation::DelegateToMixnode => f.write_str("DelegateToMixnode"),
            Operation::DelegateToMixnodeOnBehalf => f.write_str("DelegateToMixnodeOnBehalf"),
            Operation::UndelegateFromMixnode => f.write_str("UndelegateFromMixnode"),
            Operation::UndelegateFromMixnodeOnBehalf => {
                f.write_str("UndelegateFromMixnodeOnBehalf")
            }
            Operation::UpdateContractSettings => f.write_str("UpdateContractSettings"),
            Operation::BeginMixnodeRewarding => f.write_str("BeginMixnodeRewarding"),
            Operation::FinishMixnodeRewarding => f.write_str("FinishMixnodeRewarding"),
            Operation::TrackUnbondGateway => f.write_str("TrackUnbondGateway"),
            Operation::TrackUnbondMixnode => f.write_str("TrackUnbondMixnode"),
            Operation::WithdrawVestedCoins => f.write_str("WithdrawVestedCoins"),
            Operation::TrackUndelegation => f.write_str("TrackUndelegation"),
            Operation::CreatePeriodicVestingAccount => f.write_str("CreatePeriodicVestingAccount"),
            Operation::AdvanceCurrentInterval => f.write_str("AdvanceCurrentInterval"),
            Operation::WriteRewardedSet => f.write_str("WriteRewardedSet"),
            Operation::ClearRewardedSet => f.write_str("ClearRewardedSet"),
            Operation::UpdateMixnetAddress => f.write_str("UpdateMixnetAddress"),
        }
    }
}

impl Operation {
    // TODO: some value tweaking
    pub fn default_gas_limit(&self) -> Gas {
        match self {
            Operation::Upload => 3_000_000u64.into(),
            Operation::Init => 500_000u64.into(),
            Operation::Migrate => 200_000u64.into(),
            Operation::ChangeAdmin => 80_000u64.into(),
            Operation::Send => 80_000u64.into(),

            Operation::BondMixnode => 175_000u64.into(),
            Operation::BondMixnodeOnBehalf => 200_000u64.into(),
            Operation::UnbondMixnode => 175_000u64.into(),
            Operation::UnbondMixnodeOnBehalf => 175_000u64.into(),
            Operation::UpdateMixnodeConfig => 175_000u64.into(),
            Operation::DelegateToMixnode => 175_000u64.into(),
            Operation::DelegateToMixnodeOnBehalf => 175_000u64.into(),
            Operation::UndelegateFromMixnode => 175_000u64.into(),
            Operation::UndelegateFromMixnodeOnBehalf => 175_000u64.into(),

            Operation::BondGateway => 175_000u64.into(),
            Operation::BondGatewayOnBehalf => 200_000u64.into(),
            Operation::UnbondGateway => 175_000u64.into(),
            Operation::UnbondGatewayOnBehalf => 200_000u64.into(),

            Operation::UpdateContractSettings => 175_000u64.into(),
            Operation::BeginMixnodeRewarding => 175_000u64.into(),
            Operation::FinishMixnodeRewarding => 175_000u64.into(),
            Operation::TrackUnbondGateway => 175_000u64.into(),
            Operation::TrackUnbondMixnode => 175_000u64.into(),
            Operation::WithdrawVestedCoins => 175_000u64.into(),
            Operation::TrackUndelegation => 175_000u64.into(),
            Operation::CreatePeriodicVestingAccount => 175_000u64.into(),
            Operation::AdvanceCurrentInterval => 175_000u64.into(),
            Operation::WriteRewardedSet => 175_000u64.into(),
            Operation::ClearRewardedSet => 175_000u64.into(),
            Operation::UpdateMixnetAddress => 80_000u64.into(),
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

    pub fn default_fee(&self, gas_price: &GasPrice) -> Fee {
        Self::determine_custom_fee(gas_price, self.default_gas_limit())
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

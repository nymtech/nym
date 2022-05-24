// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd::GasPrice;
use cosmrs::tx::{Fee, Gas};
use cosmrs::Coin;
use serde::{Deserialize, Serialize};
use std::fmt;

#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(
    test,
    ts(export, export_to = "../../../nym-wallet/src/types/rust/operation.ts")
)]
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
    AdvanceCurrentEpoch,
    WriteRewardedSet,
    ClearRewardedSet,
    UpdateMixnetAddress,
    CheckpointMixnodes,
    ReconcileDelegations,
}

pub(crate) fn calculate_fee(gas_price: &GasPrice, gas_limit: Gas) -> Coin {
    gas_price * gas_limit
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // so far literally every single variant uses the same output as if it was produced by debug
        // so unless it's explicitly required by some specific case, just use debug impl directly
        // note: I've used the match explicitly here, even though it's not required,
        // so that we'd remember to handle any future "special" cases explicitly
        // (or at least think about what we're doing when we add new Operation variant)
        #[allow(clippy::match_single_binding)]
        match *self {
            _ => <Self as std::fmt::Debug>::fmt(self, f),
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
            Operation::CheckpointMixnodes => 175_000u64.into(),
            Operation::ReconcileDelegations => 500_000u64.into(),
            Operation::AdvanceCurrentEpoch => 175_000u64.into(),
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

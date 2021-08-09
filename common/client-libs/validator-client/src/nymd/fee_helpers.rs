// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd::GasPrice;
use crate::ValidatorClientError;
use config::defaults;
use cosmos_sdk::tx::{Fee, Gas};
use cosmos_sdk::{Coin, Denom};
use cosmwasm_std::{Decimal, Uint128};
use std::str::FromStr;

pub(crate) enum Operation {
    Upload,
    Init,
    Migrate,
    ChangeAdmin,
    Send,

    BondMixnode,
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

impl Operation {
    pub(crate) fn gas_limit(&self) -> Gas {
        match self {
            Operation::Upload => 2_500_000u64.into(),
            Operation::Init => 500_000u64.into(),
            Operation::Migrate => 200_000u64.into(),
            Operation::ChangeAdmin => 80_000u64.into(),
            Operation::Send => 80_000u64.into(),

            Operation::BondMixnode => 175_000u64.into(),
        }
    }

    pub(crate) fn determine_fee(&self, gas_price: &GasPrice) -> Fee {
        // we need to know 2 of the following 3 parameters (the third one is being implicit) in order to construct Fee:
        // (source: https://docs.cosmos.network/v0.42/basics/gas-fees.html)
        // - gas price
        // - gas limit
        // - fees
        let gas_limit = self.gas_limit();
        let fee = calculate_fee(&gas_price, gas_limit);
        Fee::from_amount_and_gas(fee, gas_limit)
    }
}

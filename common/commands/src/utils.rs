// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmrs::AccountId;
use cosmwasm_std::{Addr, Coin as CosmWasmCoin, Decimal};
use log::error;
use serde::Serialize;
use std::error::Error;
use std::fmt::{Display, Formatter};
use validator_client::nyxd::Coin;

// TODO: perhaps it should be moved to some global common crate?
pub fn account_id_to_cw_addr(account_id: &AccountId) -> Addr {
    // the call to unchecked is fine here as we're converting directly from `AccountId`
    // which must have been a valid bech32 address
    Addr::unchecked(account_id.as_ref())
}

pub fn pretty_coin(coin: &Coin) -> String {
    let amount = Decimal::from_ratio(coin.amount, 1_000_000u128);
    let denom = if coin.denom.starts_with('u') {
        &coin.denom[1..]
    } else {
        &coin.denom
    };
    format!("{amount} {denom}")
}

pub fn pretty_cosmwasm_coin(coin: &CosmWasmCoin) -> String {
    let amount = Decimal::from_ratio(coin.amount, 1_000_000u128);
    let denom = if coin.denom.starts_with('u') {
        &coin.denom[1..]
    } else {
        &coin.denom
    };
    format!("{amount} {denom}")
}

pub fn pretty_decimal_with_denom(value: Decimal, denom: &str) -> String {
    // TODO: we might have to truncate the value here (that's why I moved it to separate function)
    format!("{value} {denom}")
}

pub fn show_error<E>(e: E)
where
    E: Display,
{
    error!("{}", e);
}

pub fn show_error_passthrough<E>(e: E) -> E
where
    E: Error + Display,
{
    error!("{}", e);
    e
}

#[derive(Serialize)]
pub(crate) struct DataWrapper<T> {
    data: T,
}

impl<T> Display for DataWrapper<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.data)
    }
}

impl<T> DataWrapper<T> {
    pub(crate) fn new(data: T) -> Self {
        DataWrapper { data }
    }
}

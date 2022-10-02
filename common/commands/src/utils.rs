// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::error::Error;
use std::fmt::Display;

use cosmwasm_std::{Coin as CosmWasmCoin, Decimal};
use log::error;
use validator_client::nymd::Coin;

pub fn pretty_coin(coin: &Coin) -> String {
    let amount = Decimal::from_ratio(coin.amount, 1_000_000u128);
    let denom = if coin.denom.starts_with('u') {
        &coin.denom[1..]
    } else {
        &coin.denom
    };
    format!("{} {}", amount, denom)
}

pub fn pretty_cosmwasm_coin(coin: &CosmWasmCoin) -> String {
    let amount = Decimal::from_ratio(coin.amount, 1_000_000u128);
    let denom = if coin.denom.starts_with('u') {
        &coin.denom[1..]
    } else {
        &coin.denom
    };
    format!("{} {}", amount, denom)
}

pub fn pretty_decimal_with_denom(value: Decimal, denom: &str) -> String {
    // TODO: we might have to truncate the value here (that's why I moved it to separate function)
    format!("{} {}", value, denom)
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

// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
use config::defaults::DENOM;
use cosmwasm_std::Coin;

pub mod events;
pub mod messages;

pub fn one_ucoin() -> Coin {
    Coin::new(1, DENOM)
}

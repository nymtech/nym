// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmrs::tx;

pub mod gas_price;
pub mod helpers;

pub enum Fee {
    Manual(tx::Fee),
    Auto(Option<f32>),
}

impl From<tx::Fee> for Fee {
    fn from(fee: tx::Fee) -> Self {
        Fee::Manual(fee)
    }
}

impl From<f32> for Fee {
    fn from(multiplier: f32) -> Self {
        Fee::Auto(Some(multiplier))
    }
}

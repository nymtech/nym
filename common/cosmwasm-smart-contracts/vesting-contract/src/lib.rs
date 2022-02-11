// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
use config::defaults::DENOM;
use cosmwasm_std::{Coin, Timestamp};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod events;
pub mod messages;

pub fn one_ucoin() -> Coin {
    Coin::new(1, DENOM)
}

#[cfg_attr(feature = "ts-rs", derive(ts_rs::TS))]
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, JsonSchema)]
pub enum Period {
    Before,
    In(usize),
    After,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PledgeData {
    amount: Coin,
    block_time: Timestamp,
}

impl PledgeData {
    pub fn amount(&self) -> Coin {
        self.amount.clone()
    }

    pub fn block_time(&self) -> Timestamp {
        self.block_time
    }

    pub fn new(amount: Coin, block_time: Timestamp) -> Self {
        Self { amount, block_time }
    }
}

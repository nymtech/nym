use std::str::FromStr;

// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
use cosmwasm_std::{Addr, Coin, Timestamp, Uint128};
use mixnet_contract_common::MixId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use messages::{ExecuteMsg, InitMsg, MigrateMsg, QueryMsg};

pub mod events;
pub mod messages;

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/Period.ts")
)]
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, JsonSchema)]
pub enum Period {
    Before,
    In(usize),
    After,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PledgeData {
    pub amount: Coin,
    pub block_time: Timestamp,
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

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum PledgeCap {
    Percent(u8),
    Absolute(Uint128),
}

impl FromStr for PledgeCap {
    type Err = String;

    fn from_str(cap: &str) -> Result<Self, Self::Err> {
        match cap.parse::<f32>() {
            Ok(f) => {
                if f > 0. && f <= 1. {
                    Ok(PledgeCap::Percent((f * 100.) as u8))
                } else {
                    Err(format!("Floats have to be > 0. and <= 1., got {}", f))
                }
            }
            Err(_) => match cap.parse::<u128>() {
                Ok(i) => Ok(PledgeCap::Absolute(Uint128::from(i))),
                Err(_e) => Err(format!("Could not parse {} as f32 or u128", cap)),
            },
        }
    }
}

impl Default for PledgeCap {
    fn default() -> Self {
        PledgeCap::Absolute(Uint128::from(100_000_000_000u128))
    }
}

impl PledgeCap {
    pub fn valid(&self) -> bool {
        match &self {
            PledgeCap::Percent(p) => *p > 0 && *p <= 100,
            PledgeCap::Absolute(_) => true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OriginalVestingResponse {
    pub amount: Coin,
    pub number_of_periods: usize,
    pub period_duration: u64,
}

impl OriginalVestingResponse {
    pub fn amount(&self) -> Coin {
        self.amount.clone()
    }

    pub fn number_of_periods(&self) -> usize {
        self.number_of_periods
    }

    pub fn period_duration(&self) -> u64 {
        self.period_duration
    }

    pub fn new(amount: Coin, number_of_periods: usize, period_duration: u64) -> Self {
        Self {
            amount,
            number_of_periods,
            period_duration,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, JsonSchema)]
pub struct VestingDelegation {
    pub account_id: u32,
    pub mix_id: MixId,
    pub block_timestamp: u64,
    pub amount: Uint128,
}

impl VestingDelegation {
    pub fn storage_key(&self) -> (u32, MixId, u64) {
        (self.account_id, self.mix_id, self.block_timestamp)
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, JsonSchema)]
pub struct DelegationTimesResponse {
    pub owner: Addr,
    pub account_id: u32,
    pub mix_id: MixId,
    pub delegation_timestamps: Vec<u64>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, JsonSchema)]
pub struct AllDelegationsResponse {
    pub delegations: Vec<VestingDelegation>,
    pub start_next_after: Option<(u32, MixId, u64)>,
}

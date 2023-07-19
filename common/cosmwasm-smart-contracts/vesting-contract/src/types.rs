// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use contracts_common::Percent;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, Timestamp, Uint128};
use mixnet_contract_common::MixId;
use std::str::FromStr;

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/Period.ts")
)]
#[cw_serde]
pub enum Period {
    Before,
    In(usize),
    After,
}

#[cw_serde]
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

#[cw_serde]
pub enum PledgeCap {
    Percent(Percent),
    Absolute(Uint128), // This has to be in unym
}

impl FromStr for PledgeCap {
    type Err = String;

    fn from_str(cap: &str) -> Result<Self, Self::Err> {
        let cap = cap.replace('_', "").replace(',', ".");
        match Percent::from_str(&cap) {
            Ok(p) => Ok(PledgeCap::Percent(p)),
            Err(_) => match cap.parse::<u128>() {
                Ok(i) => Ok(PledgeCap::Absolute(Uint128::from(i))),
                Err(_e) => Err(format!("Could not parse {cap} as Percent or Uint128")),
            },
        }
    }
}

impl Default for PledgeCap {
    fn default() -> Self {
        #[allow(clippy::expect_used)]
        PledgeCap::Percent(Percent::from_percentage_value(10).expect("This can never fail!"))
    }
}

#[cw_serde]
pub struct VestingPeriod {
    pub start_time: u64,
    pub period_seconds: u64,
}

impl VestingPeriod {
    pub fn end_time(&self) -> Timestamp {
        Timestamp::from_seconds(self.start_time + self.period_seconds)
    }
}

#[cw_serde]
#[derive(Default)]
pub struct VestingSpecification {
    start_time: Option<u64>,
    period_seconds: Option<u64>,
    num_periods: Option<u64>,
}

impl VestingSpecification {
    pub fn new(
        start_time: Option<u64>,
        period_seconds: Option<u64>,
        num_periods: Option<u64>,
    ) -> Self {
        Self {
            start_time,
            period_seconds,
            num_periods,
        }
    }

    pub fn start_time(&self) -> Option<u64> {
        self.start_time
    }

    pub fn period_seconds(&self) -> u64 {
        self.period_seconds.unwrap_or(3 * 30 * 86400)
    }

    pub fn num_periods(&self) -> u64 {
        self.num_periods.unwrap_or(8)
    }

    pub fn populate_vesting_periods(&self, start_time: u64) -> Vec<VestingPeriod> {
        let mut periods = Vec::with_capacity(self.num_periods() as usize);
        for i in 0..self.num_periods() {
            let period = VestingPeriod {
                start_time: start_time + i * self.period_seconds(),
                period_seconds: self.period_seconds(),
            };
            periods.push(period);
        }
        periods
    }
}

#[cw_serde]
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

// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use contracts_common::Percent;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, Timestamp, Uint128};
use mixnet_contract_common::NodeId;
use std::str::FromStr;

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export, export_to = "ts-packages/types/src/types/rust/Period.ts")
)]
#[cw_serde]
/// The vesting period.
pub enum Period {
    /// Defines a pre-vesting period.
    #[serde(alias = "Before")]
    Before,

    /// Defines currently active vesting period.
    #[serde(alias = "In")]
    In(usize),

    /// Defines a post-vesting period.
    #[serde(alias = "After")]
    After,
}

/// Information regarding pledge (i.e. mixnode or gateway bonding) made with vesting tokens.
#[cw_serde]
pub struct PledgeData {
    /// The amount pledged.
    pub amount: Coin,

    /// The block timestamp where the pledge occurred.
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

/// Defines cap for pleding/staking tokens.
#[cw_serde]
pub enum PledgeCap {
    /// Specifies a percent-based pledge cap, i.e. only given % of tokens could be pledged/staked.
    #[serde(alias = "Percent")]
    Percent(Percent),

    /// Specifies an absolute pledge cap, i.e. an explicit value that could be pledged/staked.
    #[serde(alias = "Absolute")]
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

/// Vesting period details.
#[cw_serde]
pub struct VestingPeriod {
    /// The start time of this vesting period, as unix timestamp.
    pub start_time: u64,

    /// The duration (in seconds) of the vesting period.
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

/// Details about particular vesting delegation.
#[cw_serde]
pub struct VestingDelegation {
    /// The id of the vesting account that has made the delegation.
    pub account_id: u32,

    /// The id of the mixnode towards which the delegation has been made.
    pub mix_id: NodeId,

    /// The block timestamp when the delegation has been made.
    pub block_timestamp: u64,

    /// The raw amount delegated (interpreted to be in the same denom as the underlying vesting specification)
    pub amount: Uint128,
}

impl VestingDelegation {
    pub fn storage_key(&self) -> (u32, NodeId, u64) {
        (self.account_id, self.mix_id, self.block_timestamp)
    }
}

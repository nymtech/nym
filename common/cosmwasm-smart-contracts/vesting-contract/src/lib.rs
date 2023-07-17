// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]

use contracts_common::Percent;
use cosmwasm_std::{Addr, Coin, Timestamp, Uint128};
use mixnet_contract_common::MixId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
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
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
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

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct AccountVestingCoins {
    pub account_id: u32,
    pub owner: Addr,
    pub still_vesting: Coin,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct VestingCoinsResponse {
    pub accounts: Vec<AccountVestingCoins>,
    pub start_next_after: Option<Addr>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct BaseVestingAccountInfo {
    pub account_id: u32,
    pub owner: Addr,
    // TODO: should this particular query/response expose anything else?
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct AccountsResponse {
    pub accounts: Vec<BaseVestingAccountInfo>,
    pub start_next_after: Option<Addr>,
}

#[cfg(test)]
mod test {
    use contracts_common::Percent;
    use cosmwasm_std::Uint128;
    use std::str::FromStr;

    use crate::PledgeCap;

    #[test]
    fn test_pledge_cap_from_str() {
        assert_eq!(
            PledgeCap::from_str("0.1").unwrap(),
            PledgeCap::Percent(Percent::from_percentage_value(10).unwrap())
        );
        assert_eq!(
            PledgeCap::from_str("0,1").unwrap(),
            PledgeCap::Percent(Percent::from_percentage_value(10).unwrap())
        );
        assert_eq!(
            PledgeCap::from_str("100_000_000_000").unwrap(),
            PledgeCap::Absolute(Uint128::new(100_000_000_000))
        );
        assert_eq!(
            PledgeCap::from_str("100000000000").unwrap(),
            PledgeCap::Absolute(Uint128::new(100_000_000_000))
        );
    }
}

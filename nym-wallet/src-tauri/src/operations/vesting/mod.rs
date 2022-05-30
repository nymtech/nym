use crate::coin::Coin;
use serde::{Deserialize, Serialize};
use vesting_contract::vesting::Account as VestingAccount;
use vesting_contract::vesting::VestingPeriod as VestingVestingPeriod;
use vesting_contract_common::OriginalVestingResponse as VestingOriginalVestingResponse;
use vesting_contract_common::PledgeData as VestingPledgeData;

pub mod bond;
pub mod delegate;
pub mod queries;

#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../src/types/rust/pledgedata.ts"))]
#[derive(Serialize, Deserialize, Debug)]
pub struct PledgeData {
    pub amount: Coin,
    pub block_time: u64,
}

impl From<VestingPledgeData> for PledgeData {
    fn from(data: VestingPledgeData) -> Self {
        Self {
            amount: data.amount().into(),
            block_time: data.block_time().seconds(),
        }
    }
}

impl PledgeData {
    fn and_then(data: VestingPledgeData) -> Option<Self> {
        Some(data.into())
    }
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(
    test,
    ts(export, export_to = "../src/types/rust/originalvestingresponse.ts")
)]
#[derive(Serialize, Deserialize, Debug)]
pub struct OriginalVestingResponse {
    amount: Coin,
    number_of_periods: usize,
    period_duration: u64,
}

impl From<VestingOriginalVestingResponse> for OriginalVestingResponse {
    fn from(data: VestingOriginalVestingResponse) -> Self {
        Self {
            amount: data.amount().into(),
            number_of_periods: data.number_of_periods(),
            period_duration: data.period_duration(),
        }
    }
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(
    test,
    ts(export, export_to = "../src/types/rust/vestingaccountinfo.ts")
)]
#[derive(Serialize, Deserialize, Debug)]
pub struct VestingAccountInfo {
    owner_address: String,
    staking_address: Option<String>,
    start_time: u64,
    periods: Vec<VestingPeriod>,
    coin: Coin,
}

impl From<VestingAccount> for VestingAccountInfo {
    fn from(account: VestingAccount) -> Self {
        let mut periods = Vec::new();
        for period in account.periods() {
            periods.push(period.into());
        }
        Self {
            owner_address: account.owner_address().to_string(),
            staking_address: account.staking_address().map(|a| a.to_string()),
            start_time: account.start_time().seconds(),
            periods,
            coin: account.coin().into(),
        }
    }
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../src/types/rust/vestingperiod.ts"))]
#[derive(Serialize, Deserialize, Debug)]
pub struct VestingPeriod {
    start_time: u64,
    period_seconds: u64,
}

impl From<VestingVestingPeriod> for VestingPeriod {
    fn from(period: VestingVestingPeriod) -> Self {
        Self {
            start_time: period.start_time,
            period_seconds: period.period_seconds,
        }
    }
}

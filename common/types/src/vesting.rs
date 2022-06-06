use crate::currency::DecCoin;
use cosmwasm_std::Timestamp;
use serde::{Deserialize, Serialize};
use vesting_contract::vesting::Account as VestingAccount;
use vesting_contract::vesting::VestingPeriod as VestingVestingPeriod;

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/PledgeData.ts")
)]
#[derive(Serialize, Deserialize, Debug)]
pub struct PledgeData {
    pub amount: DecCoin,
    pub block_time: u64,
}

impl PledgeData {
    pub fn new(amount: DecCoin, block_time: Timestamp) -> Self {
        PledgeData {
            amount,
            block_time: block_time.seconds(),
        }
    }
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/OriginalVestingResponse.ts")
)]
#[derive(Serialize, Deserialize, Debug)]
pub struct OriginalVestingResponse {
    amount: DecCoin,
    number_of_periods: usize,
    period_duration: u64,
}

impl OriginalVestingResponse {
    pub fn new(amount: DecCoin, number_of_periods: usize, period_duration: u64) -> Self {
        OriginalVestingResponse {
            amount,
            number_of_periods,
            period_duration,
        }
    }
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/VestingAccountInfo.ts")
)]
#[derive(Serialize, Deserialize, Debug)]
pub struct VestingAccountInfo {
    owner_address: String,
    staking_address: Option<String>,
    start_time: u64,
    periods: Vec<VestingPeriod>,
    amount: DecCoin,
}

impl VestingAccountInfo {
    pub fn new(amount: DecCoin, account: VestingAccount) -> Self {
        VestingAccountInfo {
            owner_address: account.owner_address().to_string(),
            staking_address: account.staking_address().map(|a| a.to_string()),
            start_time: account.start_time().seconds(),
            periods: account.periods().into_iter().map(Into::into).collect(),
            amount,
        }
    }
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/VestingPeriod.ts")
)]
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

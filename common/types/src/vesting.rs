use crate::currency::MajorCurrencyAmount;
use crate::error::TypesError;
use serde::{Deserialize, Serialize};
use vesting_contract::vesting::Account as VestingAccount;
use vesting_contract::vesting::VestingPeriod as VestingVestingPeriod;
use vesting_contract_common::OriginalVestingResponse as VestingOriginalVestingResponse;
use vesting_contract_common::PledgeData as VestingPledgeData;

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/PledgeData.ts")
)]
#[derive(Serialize, Deserialize, Debug)]
pub struct PledgeData {
    pub amount: MajorCurrencyAmount,
    pub block_time: u64,
}

impl TryFrom<VestingPledgeData> for PledgeData {
    type Error = TypesError;

    fn try_from(data: VestingPledgeData) -> Result<Self, Self::Error> {
        let amount: MajorCurrencyAmount = data.amount().try_into()?;
        Ok(Self {
            amount,
            block_time: data.block_time().seconds(),
        })
    }
}

impl PledgeData {
    pub fn and_then(data: VestingPledgeData) -> Option<Self> {
        data.try_into().ok()
    }
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/OriginalVestingResponse.ts")
)]
#[derive(Serialize, Deserialize, Debug)]
pub struct OriginalVestingResponse {
    amount: MajorCurrencyAmount,
    number_of_periods: usize,
    period_duration: u64,
}

impl TryFrom<VestingOriginalVestingResponse> for OriginalVestingResponse {
    type Error = TypesError;

    fn try_from(data: VestingOriginalVestingResponse) -> Result<Self, Self::Error> {
        let amount = data.amount().try_into()?;
        Ok(Self {
            amount,
            number_of_periods: data.number_of_periods(),
            period_duration: data.period_duration(),
        })
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
    amount: MajorCurrencyAmount,
}

impl TryFrom<VestingAccount> for VestingAccountInfo {
    type Error = TypesError;

    fn try_from(account: VestingAccount) -> Result<Self, Self::Error> {
        let mut periods = Vec::new();
        for period in account.periods() {
            periods.push(period.into());
        }
        let amount: MajorCurrencyAmount = account.coin().try_into()?;
        Ok(Self {
            owner_address: account.owner_address().to_string(),
            staking_address: account.staking_address().map(|a| a.to_string()),
            start_time: account.start_time().seconds(),
            periods,
            amount,
        })
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

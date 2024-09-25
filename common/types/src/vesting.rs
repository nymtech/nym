use crate::currency::{DecCoin, RegisteredCoins};
use crate::error::TypesError;
use nym_vesting_contract_common::account::Account as ContractVestingAccount;
use nym_vesting_contract_common::types::VestingPeriod as ContractVestingPeriod;
use nym_vesting_contract_common::OriginalVestingResponse as ContractOriginalVestingResponse;
use nym_vesting_contract_common::PledgeData as ContractPledgeData;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export, export_to = "ts-packages/types/src/types/rust/PledgeData.ts")
)]
#[derive(Serialize, Deserialize, Debug)]
pub struct PledgeData {
    pub amount: DecCoin,
    pub block_time: u64,
}

impl PledgeData {
    pub fn from_vesting_contract(
        pledge: ContractPledgeData,
        reg: &RegisteredCoins,
    ) -> Result<Self, TypesError> {
        Ok(PledgeData {
            amount: reg.attempt_convert_to_display_dec_coin(pledge.amount.into())?,
            block_time: pledge.block_time.seconds(),
        })
    }
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/OriginalVestingResponse.ts"
    )
)]
#[derive(Serialize, Deserialize, Debug)]
pub struct OriginalVestingResponse {
    amount: DecCoin,
    number_of_periods: usize,
    period_duration: u64,
}

impl OriginalVestingResponse {
    pub fn from_vesting_contract(
        res: ContractOriginalVestingResponse,
        reg: &RegisteredCoins,
    ) -> Result<Self, TypesError> {
        Ok(OriginalVestingResponse {
            amount: reg.attempt_convert_to_display_dec_coin(res.amount.into())?,
            number_of_periods: res.number_of_periods,
            period_duration: res.period_duration,
        })
    }
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/VestingAccountInfo.ts"
    )
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
    pub fn from_vesting_contract(
        account: ContractVestingAccount,
        reg: &RegisteredCoins,
    ) -> Result<Self, TypesError> {
        Ok(VestingAccountInfo {
            owner_address: account.owner_address().to_string(),
            staking_address: account.staking_address().map(|a| a.to_string()),
            start_time: account.start_time().seconds(),
            periods: account.periods().into_iter().map(Into::into).collect(),
            amount: reg.attempt_convert_to_display_dec_coin(account.coin.into())?,
        })
    }
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/VestingPeriod.ts"
    )
)]
#[derive(Serialize, Deserialize, Debug)]
pub struct VestingPeriod {
    start_time: u64,
    period_seconds: u64,
}

impl From<ContractVestingPeriod> for VestingPeriod {
    fn from(period: ContractVestingPeriod) -> Self {
        Self {
            start_time: period.start_time,
            period_seconds: period.period_seconds,
        }
    }
}

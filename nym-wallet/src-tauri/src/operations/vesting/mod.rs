use crate::coin::Coin;
use serde::{Deserialize, Serialize};
use vesting_contract_common::OriginalVestingResponse as VestingOriginalVestingResponse;
use vesting_contract_common::PledgeData as VestingPledgeData;

pub mod bond;
pub mod delegate;
pub mod queries;

#[cfg_attr(test, derive(ts_rs::TS))]
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

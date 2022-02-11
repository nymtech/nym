use crate::coin::Coin;
use serde::{Deserialize, Serialize};
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

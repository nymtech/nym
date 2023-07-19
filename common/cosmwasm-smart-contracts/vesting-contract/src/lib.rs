// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin};
use mixnet_contract_common::MixId;

pub mod account;
pub mod error;
pub mod events;
pub mod messages;
pub mod types;

pub use account::Account;
pub use error::VestingContractError;
pub use messages::{ExecuteMsg, InitMsg, MigrateMsg, QueryMsg};
pub use types::*;

#[cw_serde]
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

#[cw_serde]
pub struct DelegationTimesResponse {
    pub owner: Addr,
    pub account_id: u32,
    pub mix_id: MixId,
    pub delegation_timestamps: Vec<u64>,
}

#[cw_serde]
pub struct AllDelegationsResponse {
    pub delegations: Vec<VestingDelegation>,
    pub start_next_after: Option<(u32, MixId, u64)>,
}

#[cw_serde]
pub struct AccountVestingCoins {
    pub account_id: u32,
    pub owner: Addr,
    pub still_vesting: Coin,
}

#[cw_serde]
pub struct VestingCoinsResponse {
    pub accounts: Vec<AccountVestingCoins>,
    pub start_next_after: Option<Addr>,
}

#[cw_serde]
pub struct BaseVestingAccountInfo {
    pub account_id: u32,
    pub owner: Addr,
    // TODO: should this particular query/response expose anything else?
}

#[cw_serde]
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

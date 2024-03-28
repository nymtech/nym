// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin};
use mixnet_contract_common::NodeId;

pub mod account;
pub mod error;
pub mod events;
pub mod messages;
pub mod types;

pub use account::Account;
pub use error::VestingContractError;
pub use messages::{ExecuteMsg, InitMsg, MigrateMsg, QueryMsg};
pub use types::*;

/// Details about the original vesting specification used when the account was created.
#[cw_serde]
pub struct OriginalVestingResponse {
    /// The original amount that was used for the creation of this vesting account
    pub amount: Coin,

    /// The number of vesting periods that the account was created with
    pub number_of_periods: usize,

    /// Duration of each vesting period in seconds
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

/// Response containing timestamps of all delegations made towards particular mixnode by given vesting account.
#[cw_serde]
pub struct DelegationTimesResponse {
    /// Address of this account's owner
    pub owner: Addr,

    /// Id associated with this account
    pub account_id: u32,

    /// Id of the mixnode towards which the delegation was made
    pub mix_id: NodeId,

    /// All timestamps where a delegation was made
    pub delegation_timestamps: Vec<u64>,
}

/// Response containing paged list of all vesting delegations made using vesting coins.
#[cw_serde]
pub struct AllDelegationsResponse {
    /// The actual vesting delegations made.
    pub delegations: Vec<VestingDelegation>,

    /// Field indicating paging information for the following queries if the caller wishes to get further entries.
    pub start_next_after: Option<(u32, NodeId, u64)>,
}

/// Basic information regarding particular vesting account alongside the amount of vesting coins.
#[cw_serde]
pub struct AccountVestingCoins {
    /// Id associated with this account
    pub account_id: u32,

    /// Address of this account's owner
    pub owner: Addr,

    /// Coins that are still vesting belonging to this account.
    pub still_vesting: Coin,
}

/// Response containing vesting coins held in this contract
#[cw_serde]
pub struct VestingCoinsResponse {
    /// The actual accounts, and their vesting coins, returned by the query.
    pub accounts: Vec<AccountVestingCoins>,

    /// Field indicating paging information for the following queries if the caller wishes to get further entries.
    pub start_next_after: Option<Addr>,
}

/// Basic information regarding particular vesting account
#[cw_serde]
pub struct BaseVestingAccountInfo {
    /// Id associated with this account
    pub account_id: u32,

    /// Address of this account's owner
    pub owner: Addr,
    // TODO: should this particular query/response expose anything else?
}

/// Response containing basic vesting account information
#[cw_serde]
pub struct AccountsResponse {
    /// The actual accounts returned by the query.
    pub accounts: Vec<BaseVestingAccountInfo>,

    /// Field indicating paging information for the following queries if the caller wishes to get further entries.
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

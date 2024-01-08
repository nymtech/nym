// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{Period, PledgeCap, VestingContractError, VestingPeriod};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Timestamp};

// this shouldn't really be exposed like this, but we really don't want to migrate the contract just for this...
pub type VestingAccountStorageKey = u32;

/// Vesting account information.
#[cw_serde]
pub struct Account {
    /// Address of the owner of the vesting account.
    pub owner_address: Addr,

    /// Optional address of an account allowed to perform staking on behalf of the owner.
    pub staking_address: Option<Addr>,

    /// The starting vesting time.
    pub start_time: Timestamp,

    /// All vesting periods for this account.
    pub periods: Vec<VestingPeriod>,

    /// The initial amount of coins used creation of this account.
    pub coin: Coin,

    /// The id/storage_key of this vesting account.
    pub storage_key: VestingAccountStorageKey,

    /// Optional custom pledge cap of this vesting account.
    #[serde(default)]
    pub pledge_cap: Option<PledgeCap>,
}

impl Account {
    pub fn pledge_cap(&self) -> PledgeCap {
        self.pledge_cap.clone().unwrap_or_default()
    }

    pub fn coin(&self) -> Coin {
        self.coin.clone()
    }

    pub fn num_vesting_periods(&self) -> usize {
        self.periods.len()
    }

    pub fn period_duration(&self) -> Result<u64, VestingContractError> {
        self.periods
            .first()
            .ok_or(VestingContractError::UnpopulatedVestingPeriods {
                owner: self.owner_address.clone(),
            })
            .map(|p| p.period_seconds)
    }

    pub fn storage_key(&self) -> u32 {
        self.storage_key
    }

    pub fn owner_address(&self) -> Addr {
        self.owner_address.clone()
    }

    pub fn staking_address(&self) -> Option<&Addr> {
        self.staking_address.as_ref()
    }

    pub fn periods(&self) -> Vec<VestingPeriod> {
        self.periods.clone()
    }

    pub fn start_time(&self) -> Timestamp {
        self.start_time
    }

    pub fn tokens_per_period(&self) -> Result<u128, VestingContractError> {
        let amount = self.coin.amount.u128();
        if amount < self.num_vesting_periods() as u128 {
            Err(VestingContractError::ImprobableVestingAmount(amount))
        } else {
            // Remainder tokens will be lumped into the last period.
            Ok(amount / self.num_vesting_periods() as u128)
        }
    }

    /// Returns the index of the next vesting period. Unless the current time is somehow in the past or vesting has not started yet.
    /// In case vesting is over it will always return NUM_VESTING_PERIODS.
    pub fn get_current_vesting_period(
        &self,
        block_time: Timestamp,
    ) -> Result<Period, VestingContractError> {
        let first_period =
            self.periods
                .first()
                .ok_or(VestingContractError::UnpopulatedVestingPeriods {
                    owner: self.owner_address.clone(),
                })?;

        let last_period =
            self.periods
                .last()
                .ok_or(VestingContractError::UnpopulatedVestingPeriods {
                    owner: self.owner_address.clone(),
                })?;

        if block_time.seconds() < first_period.start_time {
            Ok(Period::Before)
        } else if last_period.end_time() < block_time {
            Ok(Period::After)
        } else {
            let mut index = 0;
            for period in &self.periods {
                if block_time < period.end_time() {
                    break;
                }
                index += 1;
            }
            Ok(Period::In(index))
        }
    }
}

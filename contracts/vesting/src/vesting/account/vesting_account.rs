use crate::storage::{delete_account, save_account, MIX_DENOM};
use crate::traits::VestingAccount;
use crate::vesting::account::StorableVestingAccountExt;
use cosmwasm_std::{Addr, Coin, Env, Storage, Timestamp, Uint128};
use std::cmp::min;
use vesting_contract_common::{OriginalVestingResponse, Period, VestingContractError};

use super::Account;

impl VestingAccount for Account {
    fn total_staked(&self, storage: &dyn Storage) -> Result<Uint128, VestingContractError> {
        Ok(self.total_delegations(storage)? + self.total_pledged(storage)?)
    }

    /// See [VestingAccount::locked_coins] for documentation.
    /// Returns 0 in case of underflow. Which is fine, as the amount of pledged and delegated tokens can be larger then vesting_coins due to rewards and vesting periods expiring

    // TODO: rename. it's no longer 'locked'... or is it?
    fn locked_coins(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, VestingContractError> {
        let still_vesting = self.get_vesting_coins(block_time, env, storage)?.amount;
        let staked = self.total_staked(storage)?;
        let locked_amount = still_vesting.saturating_sub(staked);

        Ok(Coin {
            amount: locked_amount,
            denom: MIX_DENOM.load(storage)?,
        })
    }

    fn spendable_coins(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, VestingContractError> {
        Ok(Coin {
            amount: self
                .load_balance(storage)?
                .saturating_sub(self.locked_coins(block_time, env, storage)?.amount),
            denom: MIX_DENOM.load(storage)?,
        })
    }

    fn spendable_vested_coins(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, VestingContractError> {
        let vested = self.get_vested_coins(block_time, env, storage)?;
        let withdrawn = self.load_withdrawn(storage)?;

        let not_withdrawn_vested = vested.amount.saturating_sub(withdrawn);
        if not_withdrawn_vested == Uint128::zero() {
            return Ok(Coin {
                denom: vested.denom,
                amount: Uint128::zero(),
            });
        }
        let spendable = self.spendable_coins(block_time, env, storage)?;

        Ok(Coin {
            denom: spendable.denom,
            // TODO: actually, is it even possible for spendable > not_withdrawn_vested?
            amount: min(spendable.amount, not_withdrawn_vested),
        })
    }

    fn spendable_reward_coins(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, VestingContractError> {
        let spendable = self.spendable_coins(block_time, env, storage)?;
        let spendable_vested = self.spendable_vested_coins(block_time, env, storage)?;

        Ok(Coin {
            denom: spendable.denom,
            // don't use saturating subs as those should never fail, thus return an error
            amount: spendable.amount.checked_sub(spendable_vested.amount)?,
        })
    }

    fn get_vested_coins(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, VestingContractError> {
        let block_time = block_time.unwrap_or(env.block.time);
        let period = self.get_current_vesting_period(block_time)?;
        let denom = MIX_DENOM.load(storage)?;

        let amount = match period {
            Period::Before => Coin {
                amount: Uint128::new(0),
                denom,
            },
            Period::In(idx) => Coin {
                amount: Uint128::new(self.tokens_per_period()? * idx as u128),
                denom,
            },
            Period::After => Coin {
                amount: self.coin.amount,
                denom,
            },
        };
        Ok(amount)
    }

    fn get_vesting_coins(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, VestingContractError> {
        Ok(Coin {
            amount: self.get_original_vesting()?.amount().amount
                - self.get_vested_coins(block_time, env, storage)?.amount,
            denom: MIX_DENOM.load(storage)?,
        })
    }

    fn get_start_time(&self) -> Timestamp {
        self.start_time
    }

    fn get_end_time(&self) -> Timestamp {
        self.periods[self.num_vesting_periods() - 1].end_time()
    }

    fn get_original_vesting(&self) -> Result<OriginalVestingResponse, VestingContractError> {
        Ok(OriginalVestingResponse::new(
            self.coin.clone(),
            self.num_vesting_periods(),
            self.period_duration()?,
        ))
    }

    fn transfer_ownership(
        &mut self,
        to_address: &Addr,
        storage: &mut dyn Storage,
    ) -> Result<(), VestingContractError> {
        delete_account(self.owner_address(), storage)?;
        to_address.clone_into(&mut self.owner_address);
        save_account(self, storage)?;
        Ok(())
    }

    fn update_staking_address(
        &mut self,
        to_address: Option<Addr>,
        storage: &mut dyn Storage,
    ) -> Result<(), VestingContractError> {
        if let Some(staking_address) = self.staking_address() {
            delete_account(staking_address.to_owned(), storage)?;
        }
        self.staking_address = to_address;
        save_account(self, storage)?;
        Ok(())
    }

    fn track_reward(
        &self,
        amount: Coin,
        storage: &mut dyn Storage,
    ) -> Result<(), VestingContractError> {
        let current_balance = self.load_balance(storage)?;
        let new_balance = current_balance + amount.amount;
        self.save_balance(new_balance, storage)?;
        Ok(())
    }

    // balance consists of:
    // - original vesting amount
    // - minus what you have staked
    // - minus what you have withdrawn
    // - plus whatever reward you have claimed
    // thus rewards = (balance + withdrawn + staked) - original vesting
    fn get_historical_vested_staking_rewards(
        &self,
        storage: &dyn Storage,
    ) -> Result<Coin, VestingContractError> {
        let balance = self.load_balance(storage)?;
        let withdrawn = self.load_withdrawn(storage)?;
        let staked = self.total_staked(storage)?;
        let original = &self.coin;
        let total = balance + withdrawn + staked;

        let rewards = Coin {
            denom: original.denom.clone(),
            amount: total.checked_sub(original.amount)?,
        };

        Ok(rewards)
    }
}

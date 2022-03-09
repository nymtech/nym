use crate::errors::ContractError;
use crate::storage::{delete_account, save_account, DELEGATIONS};
use crate::traits::VestingAccount;
use config::defaults::DENOM;
use cosmwasm_std::{Addr, Coin, Env, Order, Storage, Timestamp, Uint128};
use vesting_contract_common::{OriginalVestingResponse, Period};

use super::Account;

impl VestingAccount for Account {
    fn locked_coins(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, ContractError> {
        // Returns 0 in case of underflow.
        Ok(Coin {
            amount: Uint128::new(
                self.get_vesting_coins(block_time, env)?
                    .amount
                    .u128()
                    .checked_sub(
                        self.get_delegated_vesting(block_time, env, storage)?
                            .amount
                            .u128(),
                    )
                    .ok_or(ContractError::Underflow)?
                    .checked_sub(
                        self.get_pledged_vesting(block_time, env, storage)?
                            .amount
                            .u128(),
                    )
                    .ok_or(ContractError::Underflow)?,
            ),
            denom: DENOM.to_string(),
        })
    }

    fn spendable_coins(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, ContractError> {
        Ok(Coin {
            amount: Uint128::new(
                self.load_balance(storage)?
                    .u128()
                    .saturating_sub(self.locked_coins(block_time, env, storage)?.amount.u128()),
            ),
            denom: DENOM.to_string(),
        })
    }

    fn get_vested_coins(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
    ) -> Result<Coin, ContractError> {
        let block_time = block_time.unwrap_or(env.block.time);
        let period = self.get_current_vesting_period(block_time);

        let amount = match period {
            Period::Before => Coin {
                amount: Uint128::new(0),
                denom: DENOM.to_string(),
            },
            Period::In(idx) => Coin {
                amount: Uint128::new(self.tokens_per_period()? * idx as u128),
                denom: DENOM.to_string(),
            },
            Period::After => Coin {
                amount: self.coin.amount,
                denom: DENOM.to_string(),
            },
        };
        Ok(amount)
    }

    fn get_vesting_coins(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
    ) -> Result<Coin, ContractError> {
        Ok(Coin {
            amount: self.get_original_vesting().amount().amount
                - self.get_vested_coins(block_time, env)?.amount,
            denom: DENOM.to_string(),
        })
    }

    fn get_start_time(&self) -> Timestamp {
        self.start_time
    }

    fn get_end_time(&self) -> Timestamp {
        self.periods[(self.num_vesting_periods() - 1) as usize].end_time()
    }

    fn get_original_vesting(&self) -> OriginalVestingResponse {
        OriginalVestingResponse::new(
            self.coin.clone(),
            self.num_vesting_periods(),
            self.period_duration(),
        )
    }

    fn get_delegated_free(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, ContractError> {
        let block_time = block_time.unwrap_or(env.block.time);
        let period = self.get_current_vesting_period(block_time);
        let max_vested = self.get_vested_coins(Some(block_time), env)?;
        let start_time = match period {
            Period::Before => 0,
            Period::After => u64::MAX,
            Period::In(idx) => self.periods[idx as usize].start_time,
        };

        let coin = DELEGATIONS
            .sub_prefix(self.storage_key())
            .range(storage, None, None, Order::Ascending)
            .filter_map(|x| x.ok())
            .filter(|((_mix, block_time), _amount)| *block_time < start_time)
            .fold(Uint128::zero(), |acc, ((_mix, _block_time), amount)| {
                acc + amount
            });

        let amount = Uint128::new(coin.u128().min(max_vested.amount.u128()));

        Ok(Coin {
            amount,
            denom: DENOM.to_string(),
        })
    }

    fn get_delegated_vesting(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, ContractError> {
        let block_time = block_time.unwrap_or(env.block.time);
        let delegated_free = self.get_delegated_free(Some(block_time), env, storage)?;
        let total_delegations = self.total_delegations(storage)?;

        let amount = total_delegations - delegated_free.amount;

        Ok(Coin {
            amount,
            denom: DENOM.to_string(),
        })
    }

    fn get_pledged_free(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, ContractError> {
        let block_time = block_time.unwrap_or(env.block.time);
        let period = self.get_current_vesting_period(block_time);
        let max_vested = self.get_vested_coins(Some(block_time), env)?;
        let start_time = match period {
            Period::Before => 0,
            Period::After => u64::MAX,
            Period::In(idx) => self.periods[idx as usize].start_time,
        };

        let amount = if let Some(bond) = self
            .load_mixnode_pledge(storage)?
            .or(self.load_gateway_pledge(storage)?)
        {
            if bond.block_time().seconds() < start_time {
                bond.amount().amount
            } else {
                Uint128::zero()
            }
        } else {
            Uint128::zero()
        };

        let amount = Uint128::new(amount.u128().min(max_vested.amount.u128()));

        Ok(Coin {
            amount,
            denom: DENOM.to_string(),
        })
    }

    fn get_pledged_vesting(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, ContractError> {
        let block_time = block_time.unwrap_or(env.block.time);
        let bonded_free = self.get_pledged_free(Some(block_time), env, storage)?;

        if let Some(bond) = self
            .load_mixnode_pledge(storage)?
            .or(self.load_gateway_pledge(storage)?)
        {
            let amount = bond.amount().amount - bonded_free.amount;
            Ok(Coin {
                amount,
                denom: DENOM.to_string(),
            })
        } else {
            Ok(Coin {
                amount: Uint128::zero(),
                denom: DENOM.to_string(),
            })
        }
    }

    fn transfer_ownership(
        &mut self,
        to_address: &Addr,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        delete_account(&self.owner_address(), storage)?;
        self.owner_address = to_address.to_owned();
        save_account(self, storage)?;
        Ok(())
    }

    fn update_staking_address(
        &mut self,
        to_address: Option<Addr>,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        if let Some(staking_address) = self.staking_address() {
            delete_account(staking_address, storage)?;
        }
        self.staking_address = to_address;
        save_account(self, storage)?;
        Ok(())
    }
}

use crate::contract::NUM_VESTING_PERIODS;
use crate::errors::ContractError;
use crate::storage::{delete_account, save_account};
use crate::traits::VestingAccount;
use config::defaults::DENOM;
use cosmwasm_std::{Addr, Coin, Env, Order, Storage, Timestamp, Uint128};

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
            // We're in the first period, or the vesting has not started yet.
            0 => Coin {
                amount: Uint128::new(0),
                denom: DENOM.to_string(),
            },
            // We always have 8 vesting periods, so periods 1-7 are special
            1..=7 => Coin {
                amount: Uint128::new(self.tokens_per_period()? * period as u128),
                denom: DENOM.to_string(),
            },
            _ => Coin {
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
            amount: self.get_original_vesting().amount
                - self.get_vested_coins(block_time, env)?.amount,
            denom: DENOM.to_string(),
        })
    }

    fn get_start_time(&self) -> Timestamp {
        self.start_time
    }

    fn get_end_time(&self) -> Timestamp {
        self.periods[(NUM_VESTING_PERIODS - 1) as usize].end_time()
    }

    fn get_original_vesting(&self) -> Coin {
        self.coin.clone()
    }

    fn get_delegated_free(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, ContractError> {
        let block_time = block_time.unwrap_or(env.block.time);
        let period = self.get_current_vesting_period(block_time);
        let max_vested = self.tokens_per_period()? * period as u128;
        let start_time = self.periods[period].start_time;

        let delegations_keys = self
            .delegations()
            .keys_de(storage, None, None, Order::Ascending)
            .scan((), |_, x| x.ok())
            .filter(|(_mix, block_time)| *block_time < start_time)
            .map(|(mix, block_time)| (mix, block_time))
            .collect::<Vec<(Vec<u8>, u64)>>();

        let mut amount = Uint128::zero();
        for (mix, block_time) in delegations_keys {
            amount += self.delegations().load(storage, (&mix, block_time))?
        }
        amount = Uint128::new(amount.u128().min(max_vested));

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
        let max_vested = self.tokens_per_period()? * period as u128;
        let start_time = self.periods[period].start_time;

        let amount = if let Some(bond) = self
            .load_mixnode_pledge(storage)?
            .or(self.load_gateway_pledge(storage)?)
        {
            if bond.block_time.seconds() < start_time {
                bond.amount
            } else {
                Uint128::zero()
            }
        } else {
            Uint128::zero()
        };

        let amount = Uint128::new(amount.u128().min(max_vested));

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
            let amount = bond.amount - bonded_free.amount;
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
        self.owner_address = to_address.clone();

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

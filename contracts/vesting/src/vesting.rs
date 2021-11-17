use crate::contract::{NUM_VESTING_PERIODS, VESTING_PERIOD};
use crate::errors::ContractError;
use crate::storage::{
    get_account_balance, get_account_delegations, set_account_balance, set_account_delegations,
};
use config::defaults::{DEFAULT_MIXNET_CONTRACT_ADDRESS, DENOM};
use cosmwasm_std::{Addr, Coin, Deps, DepsMut, Env, Timestamp, Uint128};
use mixnet_contract::ExecuteMsg as MixnetExecuteMsg;
use mixnet_contract::IdentityKey;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub trait VestingAccount {
    // locked_coins returns the set of coins that are not spendable (i.e. locked),
    // defined as the vesting coins that are not delegated.
    //
    // To get spendable coins of a vesting account, first the total balance must
    // be retrieved and the locked tokens can be subtracted from the total balance.
    // Note, the spendable balance can be negative.
    fn locked_coins(&self, block_time: Timestamp, env: Env, deps: Deps) -> Coin;

    // Calculates the total spendable balance that can be sent to other accounts.
    fn spendable_coins(&self, block_time: Timestamp, env: Env, deps: Deps) -> Coin;

    fn get_vested_coins(&self, block_time: Timestamp) -> Coin;
    fn get_vesting_coins(&self, block_time: Timestamp) -> Coin;

    fn get_start_time(&self) -> Timestamp;
    fn get_end_time(&self) -> Timestamp;

    fn get_original_vesting(&self) -> Coin;
    fn get_delegated_free(&self, env: Env, deps: Deps) -> Coin;
    fn get_delegated_vesting(&self, env: Env, deps: Deps) -> Coin;
}

pub trait DelegationAccount {
    fn try_delegate_to_mixnode(
        &self,
        mix_identity: IdentityKey,
        amount: Coin,
        env: Env,
        deps: DepsMut,
    ) -> Result<(), ContractError>;

    fn try_undelegate_from_mixnode(
        &self,
        mix_identity: IdentityKey,
        deps: DepsMut,
    ) -> Result<(), ContractError>;

    // track_delegation performs internal vesting accounting necessary when
    // delegating from a vesting account. It accepts the current block time, the
    // delegation amount and balance of all coins whose denomination exists in
    // the account's original vesting balance.
    fn track_delegation(
        &self,
        block_time: Timestamp,
        mix_identity: IdentityKey,
        delegation_amount: Coin,
        deps: DepsMut,
    ) -> Result<(), ContractError>;
    // track_undelegation performs internal vesting accounting necessary when a
    // vesting account performs an undelegation.
    fn track_undelegation(
        &self,
        mix_identity: IdentityKey,
        deps: DepsMut,
    ) -> Result<(), ContractError>;
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct VestingPeriod {
    pub start_time: u64,
}

impl VestingPeriod {
    pub fn end_time(&self) -> Timestamp {
        Timestamp::from_seconds(self.start_time + VESTING_PERIOD)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PeriodicVestingAccount {
    address: Addr,
    start_time: Timestamp,
    periods: Vec<VestingPeriod>,
    coin: Coin,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DelegationData {
    amount: Uint128,
    block_time: Timestamp,
}

impl PeriodicVestingAccount {
    pub fn new(
        address: Addr,
        coin: Coin,
        start_time: Timestamp,
        periods: Vec<VestingPeriod>,
        deps: &mut DepsMut,
    ) -> Result<Self, ContractError> {
        let amount = coin.amount;
        let account = PeriodicVestingAccount {
            address,
            start_time,
            periods,
            coin,
        };
        set_account_balance(deps.storage, &account.address, amount)?;
        Ok(account)
    }

    pub fn address(&self) -> Addr {
        self.address.clone()
    }

    pub fn tokens_per_period(&self) -> u128 {
        // Remainder tokens will be lumped into the last period.
        self.coin.amount.u128() / NUM_VESTING_PERIODS as u128
    }

    fn get_next_vesting_period(&self, block_time: Timestamp) -> usize {
        // Returns the index of the next vesting period. Unless the current time is somehow in the past or vesting has not started yet.
        // In case vesting is over it will always return NUM_VESTING_PERIODS.
        self.periods
            .iter()
            .map(|period| period.start_time)
            .collect::<Vec<u64>>()
            .binary_search(&block_time.seconds())
            .unwrap()
    }

    fn get_current_vesting_period(&self, block_time: Timestamp) -> usize {
        if self.get_next_vesting_period(block_time) > 0 {
            self.get_next_vesting_period(block_time) - 1
        } else {
            0
        }
    }

    fn get_delegated_with_op(&self, op: &dyn Fn(u64, u64) -> bool, env: Env, deps: Deps) -> Coin {
        let period = self.get_current_vesting_period(env.block.time);
        if period == 0 {
            return Coin {
                amount: Uint128(0),
                denom: DENOM.to_string(),
            };
        }

        let end_time = if period >= NUM_VESTING_PERIODS as usize {
            u64::MAX
        } else {
            self.periods[period].end_time().seconds()
        };

        if let Some(delegations) = get_account_delegations(deps.storage, self.address.as_str()) {
            let mut delegated = Coin {
                amount: Uint128(0),
                denom: DENOM.to_string(),
            };
            for (_mix_identity, delegation_data) in delegations.iter() {
                for delegation in delegation_data {
                    if op(delegation.block_time.seconds(), end_time) {
                        delegated.amount += delegation.amount;
                    }
                }
            }
            delegated
        } else {
            Coin {
                amount: Uint128(0),
                denom: DENOM.to_string(),
            }
        }
    }

    fn get_balance(&self, deps: Deps) -> Uint128 {
        get_account_balance(deps.storage, &self.address).unwrap_or(Uint128(0))
    }
}

impl DelegationAccount for PeriodicVestingAccount {
    fn try_delegate_to_mixnode(
        &self,
        mix_identity: IdentityKey,
        coin: Coin,
        env: Env,
        deps: DepsMut,
    ) -> Result<(), ContractError> {
        let querier = deps.querier;
        let msg = MixnetExecuteMsg::DelegateToMixnodeOnBehalf {
            mix_identity: mix_identity.clone(),
            delegate_addr: self.address.clone(),
            coin: coin.clone(),
        };
        querier.query_wasm_smart(DEFAULT_MIXNET_CONTRACT_ADDRESS, &msg)?;
        self.track_delegation(env.block.time, mix_identity, coin, deps)?;
        Ok(())
    }

    fn try_undelegate_from_mixnode(
        &self,
        mix_identity: IdentityKey,
        deps: DepsMut,
    ) -> Result<(), ContractError> {
        let querier = deps.querier;
        let msg = MixnetExecuteMsg::UnDelegateFromMixnodeOnBehalf {
            mix_identity: mix_identity.clone(),
            delegate_addr: self.address.clone(),
        };
        querier.query_wasm_smart(DEFAULT_MIXNET_CONTRACT_ADDRESS, &msg)?;
        self.track_undelegation(mix_identity, deps)?;
        Ok(())
    }

    fn track_delegation(
        &self,
        block_time: Timestamp,
        mix_identity: IdentityKey,
        delegation_amount: Coin,
        deps: DepsMut,
    ) -> Result<(), ContractError> {
        let mut delegations = if let Some(delegations) =
            get_account_delegations(deps.storage, self.address.as_str())
        {
            delegations
        } else {
            HashMap::new()
        };
        let delegation = delegations
            .entry(mix_identity)
            .or_insert_with(Vec::new);
        delegation.push(DelegationData {
            amount: delegation_amount.amount,
            block_time,
        });
        set_account_delegations(deps.storage, &self.address, delegations)?;
        Ok(())
    }

    fn track_undelegation(
        &self,
        mix_identity: IdentityKey,
        deps: DepsMut,
    ) -> Result<(), ContractError> {
        // This has to exist in storage at this point.
        let mut delegations = get_account_delegations(deps.storage, self.address.as_str()).unwrap();
        // Since we're always removing the entire delegation we can just drop the key
        delegations.remove(&mix_identity);
        Ok(set_account_delegations(
            deps.storage,
            &self.address,
            delegations,
        )?)
    }
}

impl VestingAccount for PeriodicVestingAccount {
    fn locked_coins(&self, block_time: Timestamp, env: Env, deps: Deps) -> Coin {
        // Returns 0 in case of underflow.
        Coin {
            amount: Uint128(
                self.get_vesting_coins(block_time)
                    .amount
                    .u128()
                    .saturating_sub(self.get_delegated_vesting(env, deps).amount.u128()),
            ),
            denom: DENOM.to_string(),
        }
    }

    fn spendable_coins(&self, block_time: Timestamp, env: Env, deps: Deps) -> Coin {
        Coin {
            amount: Uint128(
                self.get_balance(deps)
                    .u128()
                    .saturating_sub(self.locked_coins(block_time, env, deps).amount.u128()),
            ),
            denom: DENOM.to_string(),
        }
    }

    fn get_vested_coins(&self, block_time: Timestamp) -> Coin {
        let period = self.get_current_vesting_period(block_time);

        match period {
            // We're in the first period, or the vesting has not started yet.
            0 => Coin {
                amount: Uint128(0),
                denom: DENOM.to_string(),
            },
            1..=7 => Coin {
                amount: Uint128(self.tokens_per_period() * period as u128),
                denom: DENOM.to_string(),
            },
            _ => Coin {
                amount: self.coin.amount,
                denom: DENOM.to_string(),
            },
        }
    }

    fn get_vesting_coins(&self, block_time: Timestamp) -> Coin {
        Coin {
            amount: Uint128(
                self.get_original_vesting().amount.u128()
                    - self.get_vested_coins(block_time).amount.u128(),
            ),
            denom: DENOM.to_string(),
        }
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

    fn get_delegated_free(&self, env: Env, deps: Deps) -> Coin {
        self.get_delegated_with_op(&lt, env, deps)
    }

    fn get_delegated_vesting(&self, env: Env, deps: Deps) -> Coin {
        self.get_delegated_with_op(&ge, env, deps)
    }
}

fn lt(x: u64, y: u64) -> bool {
    x < y
}

fn ge(x: u64, y: u64) -> bool {
    x >= y
}

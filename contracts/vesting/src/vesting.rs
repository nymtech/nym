use crate::contract::{NUM_VESTING_PERIODS, VESTING_PERIOD};
use crate::errors::ContractError;
use crate::storage::{
    get_account_balance, get_account_delegations, set_account, set_account_balance,
    set_account_delegations,
};
use config::defaults::{DEFAULT_MIXNET_CONTRACT_ADDRESS, DENOM};
use cosmwasm_std::{wasm_execute, Addr, Coin, Env, QuerierWrapper, Storage, Timestamp, Uint128};
use mixnet_contract::ExecuteMsg as MixnetExecuteMsg;
use mixnet_contract::IdentityKey;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub trait VestingAccount {
    // locked_coins returns the set of coins that are not spendable (can still be delegated tough) (i.e. locked),
    // defined as the vesting coins that are not delegated.
    //
    // To get spendable coins of a vesting account, first the total balance must
    // be retrieved and the locked tokens can be subtracted from the total balance.
    // Note, the spendable balance can be negative.
    fn locked_coins(&self, block_time: Option<Timestamp>, env: &Env, storage: &dyn Storage)
        -> Coin;

    // Calculates the total spendable balance that can be sent to other accounts.
    fn spendable_coins(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Coin;

    fn get_vested_coins(&self, block_time: Option<Timestamp>, env: &Env) -> Coin;
    fn get_vesting_coins(&self, block_time: Option<Timestamp>, env: &Env) -> Coin;

    fn get_start_time(&self) -> Timestamp;
    fn get_end_time(&self) -> Timestamp;

    fn get_original_vesting(&self) -> Coin;
    fn get_delegated_free(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Coin;
    fn get_delegated_vesting(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Coin;
}

pub trait DelegationAccount {
    fn try_delegate_to_mixnode(
        &self,
        mix_identity: IdentityKey,
        amount: Coin,
        env: &Env,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError>;

    fn try_undelegate_from_mixnode(
        &self,
        mix_identity: IdentityKey,
        storage: &mut dyn Storage,
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
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError>;
    // track_undelegation performs internal vesting accounting necessary when a
    // vesting account performs an undelegation.
    fn track_undelegation(
        &self,
        mix_identity: IdentityKey,
        storage: &mut dyn Storage,
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
    mix_identity: IdentityKey,
    amount: Uint128,
    block_time: Timestamp,
}

impl PeriodicVestingAccount {
    pub fn new(
        address: Addr,
        coin: Coin,
        start_time: Timestamp,
        periods: Vec<VestingPeriod>,
        storage: &mut dyn Storage,
    ) -> Result<Self, ContractError> {
        let amount = coin.amount;
        let account = PeriodicVestingAccount {
            address,
            start_time,
            periods,
            coin,
        };
        set_account(storage, account.clone())?;
        set_account_balance(storage, &account.address, amount)?;
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
        match self
            .periods
            .iter()
            .map(|period| period.start_time)
            .collect::<Vec<u64>>()
            .binary_search(&block_time.seconds())
        {
            Ok(u) => u,
            Err(u) => u,
        }
    }

    fn get_current_vesting_period(&self, block_time: Timestamp) -> usize {
        if self.get_next_vesting_period(block_time) > 0 {
            self.get_next_vesting_period(block_time) - 1
        } else {
            0
        }
    }

    fn get_delegated_with_op(
        &self,
        op: &dyn Fn(u64, u64) -> bool,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Coin {
        let block_time = block_time.unwrap_or(env.block.time);
        let period = self.get_current_vesting_period(block_time);
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

        if let Some(delegations) = get_account_delegations(storage, &self.address) {
            delegations
                .iter()
                .filter(|d| op(d.block_time.seconds(), end_time))
                .fold(
                    Coin {
                        amount: Uint128(0),
                        denom: DENOM.to_string(),
                    },
                    |mut acc, d| {
                        acc.amount += d.amount;
                        acc
                    },
                )
        } else {
            Coin {
                amount: Uint128(0),
                denom: DENOM.to_string(),
            }
        }
    }

    fn get_balance(&self, storage: &dyn Storage) -> Uint128 {
        get_account_balance(storage, &self.address).unwrap_or(Uint128(0))
    }
}

impl DelegationAccount for PeriodicVestingAccount {
    fn try_delegate_to_mixnode(
        &self,
        mix_identity: IdentityKey,
        coin: Coin,
        env: &Env,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        if coin.amount < self.get_balance(storage) {
            let msg = MixnetExecuteMsg::DelegateToMixnodeOnBehalf {
                mix_identity: mix_identity.clone(),
                delegate_addr: self.address.clone(),
            };
            wasm_execute(DEFAULT_MIXNET_CONTRACT_ADDRESS, &msg, vec![coin.clone()])?;
            self.track_delegation(env.block.time, mix_identity, coin, storage)?;

            Ok(())
        } else {
            return Err(ContractError::InsufficientBalance(
                self.address.as_str().to_string(),
                self.get_balance(storage).u128(),
            ));
        }
    }

    fn try_undelegate_from_mixnode(
        &self,
        mix_identity: IdentityKey,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        let msg = MixnetExecuteMsg::UnDelegateFromMixnodeOnBehalf {
            mix_identity: mix_identity.clone(),
            delegate_addr: self.address.clone(),
        };
        wasm_execute(
            DEFAULT_MIXNET_CONTRACT_ADDRESS,
            &msg,
            vec![Coin {
                amount: Uint128(0),
                denom: DENOM.to_string(),
            }],
        )?;
        self.track_undelegation(mix_identity, storage)?;
        Ok(())
    }

    fn track_delegation(
        &self,
        block_time: Timestamp,
        mix_identity: IdentityKey,
        delegation_amount: Coin,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        let mut delegations =
            if let Some(delegations) = get_account_delegations(storage, &self.address) {
                delegations
            } else {
                Vec::new()
            };
        delegations.push(DelegationData {
            mix_identity,
            amount: delegation_amount.amount,
            block_time,
        });
        // TODO: track balance here as well.
        set_account_delegations(storage, &self.address, delegations)?;
        Ok(())
    }

    fn track_undelegation(
        &self,
        mix_identity: IdentityKey,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        // This has to exist in storage at this point.
        let delegations = get_account_delegations(storage, &self.address)
            .unwrap()
            .into_iter()
            .filter(|d| d.mix_identity != mix_identity)
            .collect();
        // Since we're always removing the entire delegation we can just drop the key
        // TODO: track balance here as well.
        Ok(set_account_delegations(
            storage,
            &self.address,
            delegations,
        )?)
    }
}

impl VestingAccount for PeriodicVestingAccount {
    fn locked_coins(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Coin {
        // Returns 0 in case of underflow.
        Coin {
            amount: Uint128(
                self.get_vesting_coins(block_time, env)
                    .amount
                    .u128()
                    .saturating_sub(
                        self.get_delegated_vesting(block_time, env, storage)
                            .amount
                            .u128(),
                    ),
            ),
            denom: DENOM.to_string(),
        }
    }

    fn spendable_coins(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Coin {
        Coin {
            amount: Uint128(
                self.get_balance(storage)
                    .u128()
                    .saturating_sub(self.locked_coins(block_time, env, storage).amount.u128()),
            ),
            denom: DENOM.to_string(),
        }
    }

    fn get_vested_coins(&self, block_time: Option<Timestamp>, env: &Env) -> Coin {
        let block_time = block_time.unwrap_or(env.block.time);
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

    fn get_vesting_coins(&self, block_time: Option<Timestamp>, env: &Env) -> Coin {
        Coin {
            amount: Uint128(
                self.get_original_vesting().amount.u128()
                    - self.get_vested_coins(block_time, env).amount.u128(),
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

    fn get_delegated_free(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Coin {
        self.get_delegated_with_op(&lt, block_time, env, storage)
    }

    fn get_delegated_vesting(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Coin {
        self.get_delegated_with_op(&ge, block_time, env, storage)
    }
}

fn lt(x: u64, y: u64) -> bool {
    x < y
}

fn ge(x: u64, y: u64) -> bool {
    x >= y
}

pub fn populate_vesting_periods(start_time: u64, n: u64) -> Vec<VestingPeriod> {
    let mut periods = Vec::new();
    // There are eight 3 month periods in two years
    for i in 0..n {
        let period = VestingPeriod {
            start_time: start_time + i * VESTING_PERIOD,
        };
        periods.push(period);
    }
    periods.shrink_to_fit();
    periods
}

#[cfg(test)]
mod tests {
    use crate::contract::{NUM_VESTING_PERIODS, VESTING_PERIOD};
    use crate::storage::{get_account, get_account_balance, get_account_delegations};
    use crate::support::tests::helpers::{init_contract, vesting_account_fixture};
    use crate::vesting::{DelegationData, VestingAccount};
    use config::defaults::DENOM;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{Addr, Coin, Timestamp, Uint128};

    use super::DelegationAccount;

    #[test]
    fn test_account_creation() {
        let mut deps = init_contract();
        let env = mock_env();
        let account = vesting_account_fixture(&mut deps.storage, &env);
        let created_account = get_account(&deps.storage, &account.address);
        let created_account_test = get_account(&deps.storage, &Addr::unchecked("fixture"));
        assert_eq!(Some(&account), created_account.as_ref());
        assert_eq!(Some(&account), created_account_test.as_ref());
        assert_eq!(
            get_account_balance(&deps.storage, &account.address),
            Some(Uint128(1_000_000_000_000))
        );
        assert_eq!(
            account.get_balance(&deps.storage),
            Uint128(1_000_000_000_000)
        )
    }

    #[test]
    fn test_period_logic() {
        let mut deps = init_contract();
        let env = mock_env();

        let account = vesting_account_fixture(&mut deps.storage, &env);

        assert_eq!(account.periods.len(), NUM_VESTING_PERIODS as usize);
        assert_eq!(account.periods.len(), 8);

        let current_period = account.get_current_vesting_period(Timestamp::from_seconds(0));
        let next_period = account.get_next_vesting_period(Timestamp::from_seconds(0));
        assert_eq!(0, current_period);
        assert_eq!(0, next_period);

        let block_time = Timestamp::from_seconds(account.start_time.seconds() + VESTING_PERIOD + 1);
        let current_period = account.get_current_vesting_period(block_time);
        let next_period = account.get_next_vesting_period(block_time);
        assert_eq!(current_period, 1);
        assert_eq!(next_period, 2);
        let vested_coins = account.get_vested_coins(Some(block_time), &env);
        let vesting_coins = account.get_vesting_coins(Some(block_time), &env);
        assert_eq!(
            vested_coins.amount,
            Uint128(account.get_original_vesting().amount.u128() / NUM_VESTING_PERIODS as u128)
        );
        assert_eq!(
            vesting_coins.amount,
            Uint128(
                account.get_original_vesting().amount.u128()
                    - account.get_original_vesting().amount.u128() / NUM_VESTING_PERIODS as u128
            )
        );

        let block_time =
            Timestamp::from_seconds(account.start_time.seconds() + 5 * VESTING_PERIOD + 1);
        let current_period = account.get_current_vesting_period(block_time);
        let next_period = account.get_next_vesting_period(block_time);
        assert_eq!(current_period, 5);
        assert_eq!(next_period, 6);
        let vested_coins = account.get_vested_coins(Some(block_time), &env);
        let vesting_coins = account.get_vesting_coins(Some(block_time), &env);
        assert_eq!(
            vested_coins.amount,
            Uint128(5 * account.get_original_vesting().amount.u128() / NUM_VESTING_PERIODS as u128)
        );
        assert_eq!(
            vesting_coins.amount,
            Uint128(
                account.get_original_vesting().amount.u128()
                    - 5 * account.get_original_vesting().amount.u128()
                        / NUM_VESTING_PERIODS as u128
            )
        );
    }

    #[test]
    fn test_toodelegations() {
        let mut deps = init_contract();
        let env = mock_env();

        let account = vesting_account_fixture(&mut deps.storage, &env);

        // Try delegating too much
        let err = account.try_delegate_to_mixnode(
            "alice".to_string(),
            Coin {
                amount: Uint128(1_000_000_000_001),
                denom: DENOM.to_string(),
            },
            &env,
            &mut deps.storage,
        );
        assert!(err.is_err());

        let ok = account.try_delegate_to_mixnode(
            "alice".to_string(),
            Coin {
                amount: Uint128(100_000_000_000),
                denom: DENOM.to_string(),
            },
            &env,
            &mut deps.storage,
        );
        assert!(ok.is_ok());

        let delegations = get_account_delegations(&mut deps.storage, &account.address).unwrap();
        assert_eq!(
            DelegationData {
                mix_identity: "alice".to_string(),
                block_time: env.block.time,
                amount: Uint128(100_000_000_000)
            },
            delegations[0]
        );
    }
}

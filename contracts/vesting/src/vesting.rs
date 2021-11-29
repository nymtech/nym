use crate::contract::{NUM_VESTING_PERIODS, VESTING_PERIOD, save_account};
use crate::errors::ContractError;
use config::defaults::{DEFAULT_MIXNET_CONTRACT_ADDRESS, DENOM};
use cosmwasm_std::{wasm_execute, Addr, Coin, Env, Order, Response, Storage, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};
use mixnet_contract::IdentityKey;
use mixnet_contract::{ExecuteMsg as MixnetExecuteMsg, MixNode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub trait VestingAccount {
    // locked_coins returns the set of coins that are not spendable (can still be delegated tough) (i.e. locked),
    // defined as the vesting coins that are not delegated.
    //
    // To get spendable coins of a vesting account, first the total balance must
    // be retrieved and the locked tokens can be subtracted from the total balance.
    // Note, the spendable balance can be negative.
    fn locked_coins(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, ContractError>;

    // Calculates the total spendable balance that can be sent to other accounts.
    fn spendable_coins(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, ContractError>;

    fn get_vested_coins(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
    ) -> Result<Coin, ContractError>;
    fn get_vesting_coins(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
    ) -> Result<Coin, ContractError>;

    fn get_start_time(&self) -> Timestamp;
    fn get_end_time(&self) -> Timestamp;

    fn get_original_vesting(&self) -> Coin;
    fn get_delegated_free(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, ContractError>;
    fn get_delegated_vesting(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, ContractError>;
}

pub trait DelegationAccount {
    fn try_delegate_to_mixnode(
        &self,
        mix_identity: IdentityKey,
        amount: Coin,
        env: &Env,
        storage: &mut dyn Storage,
    ) -> Result<Response, ContractError>;

    fn try_undelegate_from_mixnode(
        &self,
        mix_identity: IdentityKey,
    ) -> Result<Response, ContractError>;

    // track_delegation performs internal vesting accounting necessary when
    // delegating from a vesting account. It accepts the current block time, the
    // delegation amount and balance of all coins whose denomination exists in
    // the account's original vesting balance.
    fn track_delegation(
        &self,
        block_time: Timestamp,
        mix_identity: IdentityKey,
        delegation: Coin,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError>;
    // track_undelegation performs internal vesting accounting necessary when a
    // vesting account performs an undelegation.
    fn track_undelegation(
        &self,
        mix_identity: IdentityKey,
        amount: Coin,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError>;
}

pub trait BondingAccount {
    fn try_bond_mixnode(
        &self,
        mix_node: MixNode,
        amount: Coin,
        env: &Env,
        storage: &mut dyn Storage,
    ) -> Result<Response, ContractError>;

    fn try_unbond_mixnode(&self) -> Result<Response, ContractError>;

    fn track_bond(
        &self,
        block_time: Timestamp,
        bond: Coin,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError>;

    fn track_unbond(&self, amount: Coin, storage: &mut dyn Storage) -> Result<(), ContractError>;
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

const DELEGATIONS_SUFFIX: &str = "de";
const BALANCE_SUFFIX: &str = "ba";
const BOND_SUFFIX: &str = "bo";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PeriodicVestingAccount {
    address: Addr,
    start_time: Timestamp,
    periods: Vec<VestingPeriod>,
    coin: Coin,
    delegations_key: String,
    balance_key: String,
    bond_key: String,
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
            address: address.clone(),
            start_time,
            periods,
            coin,
            delegations_key: format!("{}_{}", address.to_string(), DELEGATIONS_SUFFIX),
            balance_key: format!("{}_{}", address.to_string(), BALANCE_SUFFIX),
            bond_key: format!("{}_{}", address.to_string(), BOND_SUFFIX),
        };
        save_account(&account, storage)?;
        account.save_balance(amount, storage)?;
        Ok(account)
    }

    pub fn address(&self) -> Addr {
        self.address.clone()
    }

    pub fn tokens_per_period(&self) -> Result<u128, ContractError> {
        let amount = self.coin.amount.u128();
        if amount < NUM_VESTING_PERIODS as u128 {
            Err(ContractError::ImprobableVestingAmount(amount))
        } else {
            // Remainder tokens will be lumped into the last period.
            Ok(amount / NUM_VESTING_PERIODS as u128)
        }
    }

    fn get_current_vesting_period(&self, block_time: Timestamp) -> usize {
        // Returns the index of the next vesting period. Unless the current time is somehow in the past or vesting has not started yet.
        // In case vesting is over it will always return NUM_VESTING_PERIODS.
        let period = match self
            .periods
            .iter()
            .map(|period| period.start_time)
            .collect::<Vec<u64>>()
            .binary_search(&block_time.seconds())
        {
            Ok(u) => u,
            Err(u) => u,
        };

        if period > 0 {
            period - 1
        } else {
            0
        }
    }

    fn get_delegated_with_op(
        &self,
        op: &dyn Fn(&u64, &u64) -> bool,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, ContractError> {
        let block_time = block_time.unwrap_or(env.block.time);
        let period = self.get_current_vesting_period(block_time);
        if period == 0 {
            return Ok(Coin {
                amount: Uint128::new(0),
                denom: DENOM.to_string(),
            });
        }

        let end_time = if period >= NUM_VESTING_PERIODS as usize {
            u64::MAX
        } else {
            self.periods[period].end_time().seconds()
        };

        let delegations_keys = self
            .delegations()
            .keys_de(storage, None, None, Order::Ascending)
            .scan((), |_, x| x.ok())
            .filter(|(_mix, block_time)| op(block_time, &end_time))
            .collect::<Vec<(String, u64)>>();

        let mut amount = Uint128::zero();

        for key in delegations_keys {
            amount += self.delegations().load(storage, key)?
        }

        Ok(Coin {
            amount,
            denom: DENOM.to_string(),
        })
    }

    pub fn load_balance(&self, storage: &dyn Storage) -> Result<Uint128, ContractError> {
        Ok(self
            .balance()
            .may_load(storage)?
            .unwrap_or_else(Uint128::zero))
    }

    pub fn save_balance(&self, amount: Uint128, storage: &mut dyn Storage) -> Result<(), ContractError> {
        Ok(self.balance().save(storage, &amount)?)
    }

    fn balance(&self) -> Item<Uint128> {
        Item::new(self.balance_key.as_ref())
    }

    pub fn load_bond(&self, storage: &dyn Storage) -> Result<Option<BondData>, ContractError> {
        Ok(self.bond().may_load(storage)?)
    }

    pub fn save_bond(&self, bond: BondData, storage: &mut dyn Storage) -> Result<(), ContractError> {
        Ok(self.bond().save(storage, &bond)?)
    }

    pub fn remove_bond(&self, storage: &mut dyn Storage) -> Result<(), ContractError> {
        self.bond().remove(storage);
        Ok(())
    }

    fn bond(&self) -> Item<BondData> {
        Item::new(self.bond_key.as_ref())
    }

    fn delegations(&self) -> Map<(IdentityKey, u64), Uint128> {
        Map::new(self.delegations_key.as_ref())
    }

    pub fn delegation_keys_for_mix(
        &self,
        mix: IdentityKey,
        storage: &dyn Storage,
    ) -> Vec<(IdentityKey, u64)> {
        self.delegations()
            .prefix_de(mix.clone())
            .keys_de(storage, None, None, Order::Ascending)
            // Scan will blow up on first error
            .scan((), |_, x| x.ok())
            .map(|t| (mix.clone(), t))
            .collect::<Vec<(IdentityKey, u64)>>()
    }

    #[allow(dead_code)]
    pub fn delegations_for_mix(
        &self,
        mix: IdentityKey,
        storage: &dyn Storage,
    ) -> Result<Vec<Uint128>, ContractError> {
        let keys = self.delegation_keys_for_mix(mix, storage);

        let mut delegation_amounts = Vec::new();

        for key in keys {
            delegation_amounts.push(self.delegations().load(storage, key)?)
        }

        Ok(delegation_amounts)
    }

    #[allow(dead_code)]
    pub fn total_delegations_for_mix(
        &self,
        mix: IdentityKey,
        storage: &dyn Storage,
    ) -> Result<Uint128, ContractError> {
        Ok(self
            .delegations_for_mix(mix, storage)?
            .iter()
            .fold(Uint128::zero(), |acc, x| acc + *x))
    }
}

impl DelegationAccount for PeriodicVestingAccount {
    fn try_delegate_to_mixnode(
        &self,
        mix_identity: IdentityKey,
        coin: Coin,
        env: &Env,
        storage: &mut dyn Storage,
    ) -> Result<Response, ContractError> {
        if coin.amount < self.load_balance(storage)? {
            let msg = MixnetExecuteMsg::DelegateToMixnodeOnBehalf {
                mix_identity: mix_identity.clone(),
                delegate: self.address.clone(),
            };
            let delegate_to_mixnode =
                wasm_execute(DEFAULT_MIXNET_CONTRACT_ADDRESS, &msg, vec![coin.clone()])?;
            self.track_delegation(env.block.time, mix_identity, coin, storage)?;

            Ok(Response::new()
                .add_attribute("action", "delegate to mixnode on behalf")
                .add_message(delegate_to_mixnode))
        } else {
            Err(ContractError::InsufficientBalance(
                self.address.as_str().to_string(),
                self.load_balance(storage)?.u128(),
            ))
        }
    }

    fn try_undelegate_from_mixnode(
        &self,
        mix_identity: IdentityKey,
    ) -> Result<Response, ContractError> {
        let msg = MixnetExecuteMsg::UndelegateFromMixnodeOnBehalf {
            mix_identity,
            delegate: self.address.clone(),
        };
        let undelegate_from_mixnode = wasm_execute(
            DEFAULT_MIXNET_CONTRACT_ADDRESS,
            &msg,
            vec![Coin {
                amount: Uint128::new(0),
                denom: DENOM.to_string(),
            }],
        )?;

        Ok(Response::new()
            .add_attribute("action", "undelegate to mixnode on behalf")
            .add_message(undelegate_from_mixnode))
    }

    fn track_delegation(
        &self,
        block_time: Timestamp,
        mix_identity: IdentityKey,
        delegation: Coin,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        let delegation_key = (mix_identity, block_time.seconds());

        let new_delegation = if let Some(existing_delegation) = self
            .delegations()
            .may_load(storage, delegation_key.clone())?
        {
            existing_delegation + delegation.amount
        } else {
            delegation.amount
        };

        self.delegations()
            .save(storage, delegation_key, &new_delegation)?;

        let new_balance =
            Uint128::new(self.load_balance(storage)?.u128() - delegation.amount.u128());

        self.save_balance(new_balance, storage)?;

        Ok(())
    }

    fn track_undelegation(
        &self,
        mix_identity: IdentityKey,
        amount: Coin,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        // Iterate over keys matching the prefix and remove them from the map
        let block_times = self
            .delegations()
            .prefix_de(mix_identity.clone())
            .keys_de(storage, None, None, Order::Ascending)
            // Scan will blow up on first error
            .scan((), |_, x| x.ok())
            .collect::<Vec<u64>>();

        for t in block_times {
            self.delegations()
                .remove(storage, (mix_identity.clone(), t))
        }

        let new_balance = Uint128::new(self.load_balance(storage)?.u128() + amount.amount.u128());

        self.save_balance(new_balance, storage)?;

        Ok(())
    }
}

impl BondingAccount for PeriodicVestingAccount {
    fn try_bond_mixnode(
        &self,
        mix_node: MixNode,
        bond: Coin,
        env: &Env,
        storage: &mut dyn Storage,
    ) -> Result<Response, ContractError> {
        if bond.amount < self.load_balance(storage)? {
            let msg = MixnetExecuteMsg::BondMixnodeOnBehalf {
                mix_node,
                owner: self.address.clone(),
            };
            let bond_mixnode =
                wasm_execute(DEFAULT_MIXNET_CONTRACT_ADDRESS, &msg, vec![bond.clone()])?;
            self.track_bond(env.block.time, bond, storage)?;

            Ok(Response::new()
                .add_attribute("action", "bond mixnode on behalf")
                .add_message(bond_mixnode))
        } else {
            Err(ContractError::InsufficientBalance(
                self.address.as_str().to_string(),
                self.load_balance(storage)?.u128(),
            ))
        }
    }

    fn track_bond(
        &self,
        block_time: Timestamp,
        bond: Coin,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        let bond = if let Some(_bond) = self.load_bond(storage)? {
            return Err(ContractError::AlreadyBonded(
                self.address.as_str().to_string(),
            ));
        } else {
            BondData {
                block_time,
                amount: bond.amount,
            }
        };

        let new_balance = Uint128::new(self.load_balance(storage)?.u128() - bond.amount.u128());

        self.save_balance(new_balance, storage)?;
        self.save_bond(bond, storage)?;
        Ok(())
    }

    fn try_unbond_mixnode(&self) -> Result<Response, ContractError> {
        let msg = MixnetExecuteMsg::UnbondMixnodeOnBehalf {
            owner: self.address.clone(),
        };
        let unbond_mixnode = wasm_execute(
            DEFAULT_MIXNET_CONTRACT_ADDRESS,
            &msg,
            vec![Coin {
                amount: Uint128::new(0),
                denom: DENOM.to_string(),
            }],
        )?;

        Ok(Response::new()
            .add_attribute("action", "unbond mixnode on behalf")
            .add_message(unbond_mixnode))
    }

    fn track_unbond(&self, amount: Coin, storage: &mut dyn Storage) -> Result<(), ContractError> {
        let new_balance = Uint128::new(self.load_balance(storage)?.u128() + amount.amount.u128());
        self.save_balance(new_balance, storage)?;

        self.remove_bond(storage)?;
        Ok(())
    }
}

impl VestingAccount for PeriodicVestingAccount {
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
                    .saturating_sub(
                        self.get_delegated_vesting(block_time, env, storage)?
                            .amount
                            .u128(),
                    ),
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
            amount: Uint128::new(
                self.get_original_vesting().amount.u128()
                    - self.get_vested_coins(block_time, env)?.amount.u128(),
            ),
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
        self.get_delegated_with_op(&lt, block_time, env, storage)
    }

    fn get_delegated_vesting(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, ContractError> {
        self.get_delegated_with_op(&ge, block_time, env, storage)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BondData {
    amount: Uint128,
    block_time: Timestamp,
}

fn lt(x: &u64, y: &u64) -> bool {
    x < y
}

fn ge(x: &u64, y: &u64) -> bool {
    x >= y
}

pub fn populate_vesting_periods(start_time: u64, n: usize) -> Vec<VestingPeriod> {
    let mut periods = Vec::with_capacity(n as usize);
    for i in 0..n {
        let period = VestingPeriod {
            start_time: start_time + i as u64 * VESTING_PERIOD,
        };
        periods.push(period);
    }
    periods
}

#[cfg(test)]
mod tests {
    use crate::contract::{NUM_VESTING_PERIODS, VESTING_PERIOD};
    use crate::support::tests::helpers::{init_contract, vesting_account_fixture};
    use crate::vesting::{VestingAccount, load_account, save_account};
    use config::defaults::DENOM;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{Addr, Coin, Timestamp, Uint128};

    use super::DelegationAccount;

    #[test]
    fn test_account_creation() {
        let mut deps = init_contract();
        let env = mock_env();
        let account = vesting_account_fixture(&mut deps.storage, &env);
        let created_account = load_account(&account.address, &deps.storage).unwrap();
        let created_account_test = load_account(&Addr::unchecked("fixture"), &deps.storage).unwrap();
        assert_eq!(Some(&account), created_account.as_ref());
        assert_eq!(Some(&account), created_account_test.as_ref());
        assert_eq!(
            account.load_balance(&deps.storage).unwrap(),
            Uint128::new(1_000_000_000_000)
        );
        assert_eq!(
            account.load_balance(&deps.storage).unwrap(),
            Uint128::new(1_000_000_000_000)
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
        assert_eq!(0, current_period);

        let block_time = Timestamp::from_seconds(account.start_time.seconds() + VESTING_PERIOD + 1);
        let current_period = account.get_current_vesting_period(block_time);
        assert_eq!(current_period, 1);
        let vested_coins = account.get_vested_coins(Some(block_time), &env).unwrap();
        let vesting_coins = account.get_vesting_coins(Some(block_time), &env).unwrap();
        assert_eq!(
            vested_coins.amount,
            Uint128::new(
                account.get_original_vesting().amount.u128() / NUM_VESTING_PERIODS as u128
            )
        );
        assert_eq!(
            vesting_coins.amount,
            Uint128::new(
                account.get_original_vesting().amount.u128()
                    - account.get_original_vesting().amount.u128() / NUM_VESTING_PERIODS as u128
            )
        );

        let block_time =
            Timestamp::from_seconds(account.start_time.seconds() + 5 * VESTING_PERIOD + 1);
        let current_period = account.get_current_vesting_period(block_time);
        assert_eq!(current_period, 5);
        let vested_coins = account.get_vested_coins(Some(block_time), &env).unwrap();
        let vesting_coins = account.get_vesting_coins(Some(block_time), &env).unwrap();
        assert_eq!(
            vested_coins.amount,
            Uint128::new(
                5 * account.get_original_vesting().amount.u128() / NUM_VESTING_PERIODS as u128
            )
        );
        assert_eq!(
            vesting_coins.amount,
            Uint128::new(
                account.get_original_vesting().amount.u128()
                    - 5 * account.get_original_vesting().amount.u128()
                        / NUM_VESTING_PERIODS as u128
            )
        );
    }

    #[test]
    fn test_delegations() {
        let mut deps = init_contract();
        let env = mock_env();

        let account = vesting_account_fixture(&mut deps.storage, &env);

        // Try delegating too much
        let err = account.try_delegate_to_mixnode(
            "alice".to_string(),
            Coin {
                amount: Uint128::new(1_000_000_000_001),
                denom: DENOM.to_string(),
            },
            &env,
            &mut deps.storage,
        );
        assert!(err.is_err());

        let ok = account.try_delegate_to_mixnode(
            "alice".to_string(),
            Coin {
                amount: Uint128::new(100_000_000_000),
                denom: DENOM.to_string(),
            },
            &env,
            &mut deps.storage,
        );
        assert!(ok.is_ok());

        let delegations = account
            .delegations_for_mix("alice".to_string(), &deps.storage)
            .unwrap();

        assert_eq!(Uint128::new(100_000_000_000), delegations[0]);
    }
}

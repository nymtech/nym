use super::{BondData, VestingPeriod};
use crate::contract::NUM_VESTING_PERIODS;
use crate::errors::ContractError;
use crate::storage::save_account;
use cosmwasm_std::{Addr, Coin, Order, Storage, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};
use mixnet_contract::IdentityKey;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod bonding_account;
mod delegating_account;
mod vesting_account;

const DELEGATIONS_SUFFIX: &str = "de";
const BALANCE_SUFFIX: &str = "ba";
const BOND_SUFFIX: &str = "bo";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Account {
    address: Addr,
    start_time: Timestamp,
    periods: Vec<VestingPeriod>,
    coin: Coin,
    delegations_key: String,
    balance_key: String,
    bond_key: String,
}

impl Account {
    pub fn new(
        address: Addr,
        coin: Coin,
        start_time: Timestamp,
        periods: Vec<VestingPeriod>,
        storage: &mut dyn Storage,
    ) -> Result<Self, ContractError> {
        let amount = coin.amount;
        let account = Account {
            address: address.to_owned(),
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

    #[allow(dead_code)]
    pub fn periods(&self) -> Vec<VestingPeriod> {
        self.periods.clone()
    }

    #[allow(dead_code)]
    pub fn start_time(&self) -> Timestamp {
        self.start_time
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

    pub fn get_current_vesting_period(&self, block_time: Timestamp) -> usize {
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

    pub fn load_balance(&self, storage: &dyn Storage) -> Result<Uint128, ContractError> {
        Ok(self
            .balance()
            .may_load(storage)?
            .unwrap_or_else(Uint128::zero))
    }

    pub fn save_balance(
        &self,
        amount: Uint128,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        Ok(self.balance().save(storage, &amount)?)
    }

    fn balance(&self) -> Item<Uint128> {
        Item::new(self.balance_key.as_ref())
    }

    pub fn load_bond(&self, storage: &dyn Storage) -> Result<Option<BondData>, ContractError> {
        Ok(self.bond().may_load(storage)?)
    }

    pub fn save_bond(
        &self,
        bond: BondData,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        Ok(self.bond().save(storage, &bond)?)
    }

    pub fn remove_bond(&self, storage: &mut dyn Storage) -> Result<(), ContractError> {
        self.bond().remove(storage);
        Ok(())
    }

    fn bond(&self) -> Item<BondData> {
        Item::new(self.bond_key.as_ref())
    }

    fn delegations(&self) -> Map<(&[u8], u64), Uint128> {
        Map::new(self.delegations_key.as_ref())
    }

    // Returns block_time part of the delegation key
    pub fn delegation_keys_for_mix(&self, mix: &str, storage: &dyn Storage) -> Vec<u64> {
        self.delegations()
            .prefix_de(mix.as_bytes())
            .keys_de(storage, None, None, Order::Ascending)
            // Scan will blow up on first error
            .scan((), |_, x| x.ok())
            .collect::<Vec<u64>>()
    }

    pub fn delegations_for_mix(
        &self,
        mix: IdentityKey,
        storage: &dyn Storage,
    ) -> Result<Vec<Uint128>, ContractError> {
        let mix_bytes = mix.as_bytes();
        let keys = self.delegation_keys_for_mix(&mix, storage);

        let mut delegation_amounts = Vec::new();
        for key in keys {
            delegation_amounts.push(self.delegations().load(storage, (mix_bytes, key))?)
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

    pub fn total_delegations(&self, storage: &dyn Storage) -> Result<Uint128, ContractError> {
        Ok(self
            .delegations()
            .range(storage, None, None, Order::Ascending)
            .scan((), |_, x| x.ok())
            .fold(Uint128::zero(), |acc, (_, x)| acc + x))
    }
}

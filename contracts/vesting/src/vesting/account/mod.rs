use super::{PledgeData, VestingPeriod};
use crate::contract::NUM_VESTING_PERIODS;
use crate::errors::ContractError;
use crate::storage::save_account;
use cosmwasm_std::{Addr, Coin, Order, Storage, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};
use mixnet_contract::IdentityKey;
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod delegating_account;
mod gateway_bonding_account;
mod mixnode_bonding_account;
mod vesting_account;

const DELEGATIONS_SUFFIX: &str = "de";
const BALANCE_SUFFIX: &str = "ba";
const PLEDGE_SUFFIX: &str = "bo";
const GATEWAY_SUFFIX: &str = "ga";

fn generate_storage_key(b: &[u8], storage: &dyn Storage) -> Result<String, ContractError> {
    let mut rng = StdRng::seed_from_u64(b.iter().fold(0, |acc, x| acc + *x as u64));
    // Be paranoid and check for collisions
    loop {
        let key = rng.next_u64().to_string();
        let balance_key = format!("{}{}", key, BALANCE_SUFFIX);
        let balance: Item<Uint128> = Item::new(&balance_key);
        if balance.may_load(storage)?.is_none() {
            return Ok(key);
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Account {
    owner_address: Addr,
    staking_address: Option<Addr>,
    start_time: Timestamp,
    periods: Vec<VestingPeriod>,
    coin: Coin,
    storage_key: String,
}

impl Account {
    pub fn new(
        owner_address: Addr,
        staking_address: Option<Addr>,
        coin: Coin,
        start_time: Timestamp,
        periods: Vec<VestingPeriod>,
        storage: &mut dyn Storage,
    ) -> Result<Self, ContractError> {
        let storage_key = generate_storage_key(owner_address.as_bytes(), storage)?;
        let amount = coin.amount;
        let account = Account {
            owner_address,
            staking_address,
            start_time,
            periods,
            coin,
            storage_key,
        };
        save_account(&account, storage)?;
        account.save_balance(amount, storage)?;
        Ok(account)
    }

    pub fn delegations_key(&self) -> String {
        format!("{}{}", self.storage_key, DELEGATIONS_SUFFIX)
    }

    pub fn balance_key(&self) -> String {
        format!("{}{}", self.storage_key, BALANCE_SUFFIX)
    }

    pub fn mixnode_pledge_key(&self) -> String {
        format!("{}{}", self.storage_key, PLEDGE_SUFFIX)
    }

    pub fn gateway_pledge_key(&self) -> String {
        format!("{}{}", self.storage_key, GATEWAY_SUFFIX)
    }

    pub fn owner_address(&self) -> Addr {
        self.owner_address.clone()
    }

    pub fn staking_address(&self) -> Option<&Addr> {
        self.staking_address.as_ref()
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
        let key = self.balance_key();
        let balance = Item::new(&key);
        Ok(balance.may_load(storage)?.unwrap_or_else(Uint128::zero))
    }

    pub fn save_balance(
        &self,
        amount: Uint128,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        let key = self.balance_key();
        let balance = Item::new(&key);
        Ok(balance.save(storage, &amount)?)
    }

    pub fn load_mixnode_pledge(
        &self,
        storage: &dyn Storage,
    ) -> Result<Option<PledgeData>, ContractError> {
        let key = self.mixnode_pledge_key();
        let mixnode_pledge = Item::new(&key);
        Ok(mixnode_pledge.may_load(storage)?)
    }

    pub fn save_mixnode_pledge(
        &self,
        pledge: PledgeData,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        let key = self.mixnode_pledge_key();
        let mixnode_pledge = Item::new(&key);
        Ok(mixnode_pledge.save(storage, &pledge)?)
    }

    pub fn remove_mixnode_pledge(&self, storage: &mut dyn Storage) -> Result<(), ContractError> {
        let key = self.mixnode_pledge_key();
        let mixnode_pledge: Item<PledgeData> = Item::new(&key);
        mixnode_pledge.remove(storage);
        Ok(())
    }

    pub fn load_gateway_pledge(
        &self,
        storage: &dyn Storage,
    ) -> Result<Option<PledgeData>, ContractError> {
        let key = self.gateway_pledge_key();
        let gateway_pledge = Item::new(&key);
        Ok(gateway_pledge.may_load(storage)?)
    }

    pub fn save_gateway_pledge(
        &self,
        pledge: PledgeData,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        let key = self.gateway_pledge_key();
        let gateway_pledge = Item::new(&key);
        Ok(gateway_pledge.save(storage, &pledge)?)
    }

    pub fn remove_gateway_pledge(&self, storage: &mut dyn Storage) -> Result<(), ContractError> {
        let key = self.gateway_pledge_key();
        let gateway_pledge: Item<PledgeData> = Item::new(&key);
        gateway_pledge.remove(storage);
        Ok(())
    }

    // Returns block_time part of the delegation key
    pub fn delegation_keys_for_mix(&self, mix: &str, storage: &dyn Storage) -> Vec<u64> {
        let key = self.delegations_key();
        let delegations: Map<(&[u8], u64), Uint128> = Map::new(&key);
        delegations
            .prefix_de(mix.as_bytes())
            .keys_de(storage, None, None, Order::Ascending)
            // Scan will blow up on first error
            .scan((), |_, x| x.ok())
            .collect::<Vec<u64>>()
    }

    pub fn any_delegation_for_mix(&self, mix: &str, storage: &dyn Storage) -> bool {
        !self.delegation_keys_for_mix(mix, storage).is_empty()
    }

    pub fn delegations_for_mix(
        &self,
        mix: IdentityKey,
        storage: &dyn Storage,
    ) -> Result<Vec<Uint128>, ContractError> {
        let mix_bytes = mix.as_bytes();
        let keys = self.delegation_keys_for_mix(&mix, storage);
        let delegations_key = self.delegations_key();
        let delegations: Map<(&[u8], u64), Uint128> = Map::new(&delegations_key);

        let mut delegation_amounts = Vec::new();
        for key in keys {
            delegation_amounts.push(delegations.load(storage, (mix_bytes, key))?)
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
        let delegations_key = self.delegations_key();
        let delegations: Map<(&[u8], u64), Uint128> = Map::new(&delegations_key);
        Ok(delegations
            .range(storage, None, None, Order::Ascending)
            .scan((), |_, x| x.ok())
            .fold(Uint128::zero(), |acc, (_, x)| acc + x))
    }
}

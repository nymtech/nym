use super::{PledgeData, VestingPeriod};
use crate::contract::NUM_VESTING_PERIODS;
use crate::errors::ContractError;
use crate::storage::save_account;
use cosmwasm_std::{Addr, Coin, Order, Storage, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};
use mixnet_contract::IdentityKey;
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

fn build_delegations_key(address: &Addr) -> String {
    format!("{}{}", address, DELEGATIONS_SUFFIX)
}

fn build_balance_key(address: &Addr) -> String {
    format!("{}{}", address, BALANCE_SUFFIX)
}

fn build_mixnode_pledge_key(address: &Addr) -> String {
    format!("{}{}", address, PLEDGE_SUFFIX)
}

fn build_gateway_pledge_key(address: &Addr) -> String {
    format!("{}{}", address, GATEWAY_SUFFIX)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Account {
    owner_address: Addr,
    staking_address: Option<Addr>,
    start_time: Timestamp,
    periods: Vec<VestingPeriod>,
    coin: Coin,
    delegations_key: String,
    balance_key: String,
    mixnode_pledge_key: String,
    gateway_pledge_key: String,
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
        let amount = coin.amount;
        let account = Account {
            owner_address: owner_address.to_owned(),
            staking_address,
            start_time,
            periods,
            coin,
            delegations_key: build_delegations_key(&owner_address),
            balance_key: build_balance_key(&owner_address),
            mixnode_pledge_key: build_mixnode_pledge_key(&owner_address),
            gateway_pledge_key: build_gateway_pledge_key(&owner_address),
        };
        save_account(&account, storage)?;
        account.save_balance(amount, storage)?;
        Ok(account)
    }

    fn update_delegations_key(&mut self) {
        self.delegations_key = build_delegations_key(&self.owner_address);
    }

    fn update_balance_key(&mut self) {
        self.balance_key = build_balance_key(&self.owner_address);
    }

    fn update_mixnode_pledge_key(&mut self) {
        self.mixnode_pledge_key = build_mixnode_pledge_key(&self.owner_address);
    }

    fn update_gateway_pledge_key(&mut self) {
        self.gateway_pledge_key = build_gateway_pledge_key(&self.owner_address);
    }

    fn move_delegations(&self, old_owner_address: &Addr) {
        let delegations_key = build_delegations_key(old_owner_address);
        let old_delegations: Map<(&[u8], u64), Uint128> = Map::new(&delegations_key);
        // TODO: Continue here
    }

    fn move_balance(&self, old_owner_address: &Addr) {}

    fn move_mixnode_pledge(&self, old_owner_address: &Addr) {}

    fn move_gateway_pledge(&self, old_owner_address: &Addr) {}

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

    pub fn load_mixnode_pledge(
        &self,
        storage: &dyn Storage,
    ) -> Result<Option<PledgeData>, ContractError> {
        Ok(self.mixnode_pledge().may_load(storage)?)
    }

    pub fn save_mixnode_pledge(
        &self,
        pledge: PledgeData,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        Ok(self.mixnode_pledge().save(storage, &pledge)?)
    }

    pub fn remove_mixnode_bond(&self, storage: &mut dyn Storage) -> Result<(), ContractError> {
        self.mixnode_pledge().remove(storage);
        Ok(())
    }

    fn mixnode_pledge(&self) -> Item<PledgeData> {
        Item::new(self.mixnode_pledge_key.as_ref())
    }

    pub fn load_gateway_pledge(
        &self,
        storage: &dyn Storage,
    ) -> Result<Option<PledgeData>, ContractError> {
        Ok(self.gateway_pledge().may_load(storage)?)
    }

    pub fn save_gateway_pledge(
        &self,
        pledge: PledgeData,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError> {
        Ok(self.gateway_pledge().save(storage, &pledge)?)
    }

    pub fn remove_gateway_bond(&self, storage: &mut dyn Storage) -> Result<(), ContractError> {
        self.gateway_pledge().remove(storage);
        Ok(())
    }

    fn gateway_pledge(&self) -> Item<PledgeData> {
        Item::new(self.gateway_pledge_key.as_ref())
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

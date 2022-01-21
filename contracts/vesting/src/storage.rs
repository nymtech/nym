use crate::errors::ContractError;
use crate::vesting::Account;
use crate::vesting::PledgeData;
use cosmwasm_std::Order;
use cosmwasm_std::{Addr, Api, Storage, Uint128};
use cw_storage_plus::{Item, Map};

pub const KEY: Item<u32> = Item::new("key");
const ACCOUNTS: Map<Addr, Account> = Map::new("acc");
// Holds data related to individual accounts
const BALANCES: Map<u32, Uint128> = Map::new("blc");
const BOND_PLEDGES: Map<u32, PledgeData> = Map::new("bnd");
const GATEWAY_PLEDGFES: Map<u32, PledgeData> = Map::new("gtw");
const DELEGATIONS: Map<(u32, &[u8], u64), Uint128> = Map::new("dlg");

pub fn save_delegation(
    key: (u32, &[u8], u64),
    amount: Uint128,
    storage: &mut dyn Storage,
) -> Result<(), ContractError> {
    DELEGATIONS.save(storage, key, &amount)?;
    Ok(())
}

pub fn remove_delegation(
    key: (u32, &[u8], u64),
    storage: &mut dyn Storage,
) -> Result<(), ContractError> {
    DELEGATIONS.remove(storage, key);
    Ok(())
}

#[allow(clippy::type_complexity)]
pub fn load_delegations_all(
    key: u32,
    storage: &dyn Storage,
) -> Result<Vec<((Vec<u8>, u64), Uint128)>, ContractError> {
    Ok(DELEGATIONS
        .sub_prefix(key)
        .range(storage, None, None, Order::Ascending)
        .scan((), |_, x| x.ok())
        .collect())
}

pub fn load_delegations_for_mix(
    key: u32,
    mix_identity: &str,
    storage: &dyn Storage,
) -> Result<Vec<(u64, Uint128)>, ContractError> {
    Ok(DELEGATIONS
        .prefix((key, mix_identity.as_bytes()))
        .range(storage, None, None, Order::Ascending)
        .scan((), |_, x| x.ok())
        .collect())
}

pub fn delete_account(address: &Addr, storage: &mut dyn Storage) -> Result<(), ContractError> {
    ACCOUNTS.remove(storage, address.to_owned());
    Ok(())
}

pub fn load_balance(key: u32, storage: &dyn Storage) -> Result<Uint128, ContractError> {
    Ok(BALANCES
        .may_load(storage, key)?
        .unwrap_or_else(Uint128::zero))
}

pub fn save_balance(
    key: u32,
    value: Uint128,
    storage: &mut dyn Storage,
) -> Result<(), ContractError> {
    BALANCES.save(storage, key, &value)?;
    Ok(())
}

pub fn load_bond_pledge(
    key: u32,
    storage: &dyn Storage,
) -> Result<Option<PledgeData>, ContractError> {
    Ok(BOND_PLEDGES.may_load(storage, key)?)
}

pub fn remove_bond_pledge(key: u32, storage: &mut dyn Storage) -> Result<(), ContractError> {
    BOND_PLEDGES.remove(storage, key);
    Ok(())
}

pub fn save_bond_pledge(
    key: u32,
    value: &PledgeData,
    storage: &mut dyn Storage,
) -> Result<(), ContractError> {
    BOND_PLEDGES.save(storage, key, value)?;
    Ok(())
}

pub fn load_gateway_pledge(
    key: u32,
    storage: &dyn Storage,
) -> Result<Option<PledgeData>, ContractError> {
    Ok(GATEWAY_PLEDGFES.may_load(storage, key)?)
}

pub fn save_gateway_pledge(
    key: u32,
    value: &PledgeData,
    storage: &mut dyn Storage,
) -> Result<(), ContractError> {
    GATEWAY_PLEDGFES.save(storage, key, value)?;
    Ok(())
}

pub fn remove_gateway_pledge(key: u32, storage: &mut dyn Storage) -> Result<(), ContractError> {
    GATEWAY_PLEDGFES.remove(storage, key);
    Ok(())
}

pub fn save_account(account: &Account, storage: &mut dyn Storage) -> Result<(), ContractError> {
    // This is a bit dirty, but its a simple way to allow for both staking account and owner to load it from storage
    if let Some(staking_address) = account.staking_address() {
        ACCOUNTS.save(storage, staking_address.to_owned(), account)?;
    }
    ACCOUNTS.save(storage, account.owner_address(), account)?;
    Ok(())
}

pub fn load_account(
    address: &Addr,
    storage: &dyn Storage,
) -> Result<Option<Account>, ContractError> {
    Ok(ACCOUNTS.may_load(storage, address.to_owned())?)
}

fn validate_account(address: &Addr, storage: &dyn Storage) -> Result<Account, ContractError> {
    load_account(address, storage)?
        .ok_or_else(|| ContractError::NoAccountForAddress(address.as_str().to_string()))
}

pub fn account_from_address(
    address: &str,
    storage: &dyn Storage,
    api: &dyn Api,
) -> Result<Account, ContractError> {
    validate_account(&api.addr_validate(address)?, storage)
}

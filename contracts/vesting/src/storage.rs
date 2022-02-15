use crate::errors::ContractError;
use crate::vesting::Account;
use cosmwasm_std::{Addr, Api, Storage, Uint128};
use cw_storage_plus::{Item, Map};
use mixnet_contract_common::IdentityKey;
use vesting_contract_common::PledgeData;

type BlockHeight = u64;

pub const KEY: Item<'_, u32> = Item::new("key");
const ACCOUNTS: Map<'_, String, Account> = Map::new("acc");
// Holds data related to individual accounts
const BALANCES: Map<'_, u32, Uint128> = Map::new("blc");
const BOND_PLEDGES: Map<'_, u32, PledgeData> = Map::new("bnd");
const GATEWAY_PLEDGES: Map<'_, u32, PledgeData> = Map::new("gtw");
pub const DELEGATIONS: Map<'_, (u32, IdentityKey, BlockHeight), Uint128> = Map::new("dlg");
pub const ADMIN: Item<'_, String> = Item::new("adm");
pub const MIXNET_CONTRACT_ADDRESS: Item<'_, String> = Item::new("mix");

pub fn save_delegation(
    key: (u32, IdentityKey, BlockHeight),
    amount: Uint128,
    storage: &mut dyn Storage,
) -> Result<(), ContractError> {
    DELEGATIONS.save(storage, key, &amount)?;
    Ok(())
}

pub fn remove_delegation(
    key: (u32, IdentityKey, BlockHeight),
    storage: &mut dyn Storage,
) -> Result<(), ContractError> {
    DELEGATIONS.remove(storage, key);
    Ok(())
}

pub fn delete_account(address: &Addr, storage: &mut dyn Storage) -> Result<(), ContractError> {
    ACCOUNTS.remove(storage, address.to_owned().to_string());
    Ok(())
}

pub fn load_balance(key: u32, storage: &dyn Storage) -> Result<Uint128, ContractError> {
    Ok(BALANCES
        .may_load(storage, key)
        .unwrap_or(None)
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
    Ok(BOND_PLEDGES.may_load(storage, key).unwrap_or(None))
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
    Ok(GATEWAY_PLEDGES.may_load(storage, key).unwrap_or(None))
}

pub fn save_gateway_pledge(
    key: u32,
    value: &PledgeData,
    storage: &mut dyn Storage,
) -> Result<(), ContractError> {
    GATEWAY_PLEDGES.save(storage, key, value)?;
    Ok(())
}

pub fn remove_gateway_pledge(key: u32, storage: &mut dyn Storage) -> Result<(), ContractError> {
    GATEWAY_PLEDGES.remove(storage, key);
    Ok(())
}

pub fn save_account(account: &Account, storage: &mut dyn Storage) -> Result<(), ContractError> {
    // This is a bit dirty, but its a simple way to allow for both staking account and owner to load it from storage
    if let Some(staking_address) = account.staking_address() {
        ACCOUNTS.save(storage, staking_address.to_owned().to_string(), account)?;
    }
    ACCOUNTS.save(storage, account.owner_address().to_string(), account)?;
    Ok(())
}

pub fn load_account(
    address: &Addr,
    storage: &dyn Storage,
) -> Result<Option<Account>, ContractError> {
    Ok(ACCOUNTS
        .may_load(storage, address.to_owned().to_string())
        .unwrap_or(None))
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

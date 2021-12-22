use crate::errors::ContractError;
use crate::vesting::Account;
use cosmwasm_std::{Addr, Api, Storage};
use cw_storage_plus::Map;

const ACCOUNTS: Map<Addr, Account> = Map::new("acc");

pub fn delete_account(address: &Addr, storage: &mut dyn Storage) -> Result<(), ContractError> {
    ACCOUNTS.remove(storage, address.to_owned());
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

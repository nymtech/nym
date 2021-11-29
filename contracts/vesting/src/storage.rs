use crate::errors::ContractError;
use crate::vesting::Account;
use cosmwasm_std::{Addr, Api, Storage};
use cw_storage_plus::Map;

const ACCOUNTS: Map<Addr, Account> = Map::new("acc");

pub fn save_account(account: &Account, storage: &mut dyn Storage) -> Result<(), ContractError> {
    Ok(ACCOUNTS.save(storage, account.address(), account)?)
}

pub fn load_account(
    address: &Addr,
    storage: &dyn Storage,
) -> Result<Option<Account>, ContractError> {
    Ok(ACCOUNTS.may_load(storage, address.to_owned())?)
}

pub fn validate_account(address: &Addr, storage: &dyn Storage) -> Result<Account, ContractError> {
    load_account(address, storage)?
        .ok_or_else(|| ContractError::NoAccountForAddress(address.as_str().to_string()))
}

pub fn account_from_address(
    address: &Addr,
    storage: &dyn Storage,
    api: &dyn Api,
) -> Result<Account, ContractError> {
    validate_account(&api.addr_validate(address.as_str())?, storage)
}

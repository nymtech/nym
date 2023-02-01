use crate::errors::ContractError;
use crate::vesting::Account;
use cosmwasm_std::{Addr, Api, Storage, Uint128};
use cosmwasm_std::{Order, StdResult};
use cw_storage_plus::{Item, Map};
use mixnet_contract_common::ContractVersion;
use mixnet_contract_common::{IdentityKey, MixId};
use pkg_version::*;
use vesting_contract_common::PledgeData;

const MAJOR: u32 = pkg_version_major!();
const MINOR: u32 = pkg_version_minor!();
const PATCH: u32 = pkg_version_patch!();

pub(crate) type BlockTimestampSecs = u64;
pub(crate) type AccountStorageKey = u32;

/// Counter for the unique, monotonically increasing storage key id for the vesting account data.
pub const KEY: Item<'_, AccountStorageKey> = Item::new("key");

/// Storage map containing vesting account information associated with particular owner address.
pub const ACCOUNTS: Map<'_, Addr, Account> = Map::new("acc");

/// Storage map containing information about amount of tokens associated with particular vesting account
/// that are currently present in the contract (and have not been withdrawn or staked in the mixnet contract)
// note: this assumes I understood the intent behind this correctly
const BALANCES: Map<'_, AccountStorageKey, Uint128> = Map::new("blc");

/// Storage map containing information about amount of tokens withdrawn from the contract by a particular vesting account.
const WITHDRAWNS: Map<'_, AccountStorageKey, Uint128> = Map::new("wthd");

/// Storage map containing information about amount of tokens pledged towards bonding mixnodes
/// in the mixnet contract using a particular vesting account.
const BOND_PLEDGES: Map<'_, AccountStorageKey, PledgeData> = Map::new("bnd");

/// Storage map containing information about amount of tokens pledged towards bonding gateways
/// in the mixnet contract using a particular vesting account.
const GATEWAY_PLEDGES: Map<'_, AccountStorageKey, PledgeData> = Map::new("gtw");

/// Old, pre-v2 migration, storage map that used to contain information about tokens delegated
/// towards particular mixnodes in the mixnet contract with given vesting account.
/// It should be completely empty.
pub const _OLD_DELEGATIONS: Map<'_, (AccountStorageKey, IdentityKey, BlockTimestampSecs), Uint128> =
    Map::new("dlg");

/// Storage map containing information about tokens delegated towards particular mixnodes
/// in the mixnet contract with given vesting account.
pub const DELEGATIONS: Map<'_, (AccountStorageKey, MixId, BlockTimestampSecs), Uint128> =
    Map::new("dlg_v2");

/// Explicit contract admin that is allowed, among other things, to create new vesting accounts.
pub const ADMIN: Item<'_, Addr> = Item::new("adm");

/// Address of the mixnet contract.
pub const MIXNET_CONTRACT_ADDRESS: Item<'_, Addr> = Item::new("mix");

/// The denomination of coin used for staking.
pub const MIX_DENOM: Item<'_, String> = Item::new("den");

pub fn save_delegation(
    key: (AccountStorageKey, MixId, BlockTimestampSecs),
    amount: Uint128,
    storage: &mut dyn Storage,
) -> Result<(), ContractError> {
    let existing_delegation_amount = if let Some(delegation) = DELEGATIONS.may_load(storage, key)? {
        delegation
    } else {
        Uint128::zero()
    };
    let new_delegations_amount = existing_delegation_amount + amount;
    DELEGATIONS.save(storage, key, &new_delegations_amount)?;
    Ok(())
}

pub fn remove_delegation(
    key: (AccountStorageKey, MixId, BlockTimestampSecs),
    storage: &mut dyn Storage,
) -> Result<(), ContractError> {
    DELEGATIONS.remove(storage, key);
    Ok(())
}

pub fn load_delegation_timestamps(
    prefix: (AccountStorageKey, MixId),
    storage: &dyn Storage,
) -> Result<Vec<BlockTimestampSecs>, ContractError> {
    let block_timestamps = DELEGATIONS
        .prefix(prefix)
        .keys(storage, None, None, Order::Ascending)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(block_timestamps)
}

pub fn count_subdelegations_for_mix(
    prefix: (AccountStorageKey, MixId),
    storage: &dyn Storage,
) -> u32 {
    DELEGATIONS
        .prefix(prefix)
        .keys(storage, None, None, Order::Ascending)
        .count() as u32
}

pub fn load_withdrawn(
    key: AccountStorageKey,
    storage: &dyn Storage,
) -> Result<Uint128, ContractError> {
    Ok(WITHDRAWNS
        .may_load(storage, key)
        .unwrap_or(None)
        .unwrap_or_else(Uint128::zero))
}

pub fn load_balance(
    key: AccountStorageKey,
    storage: &dyn Storage,
) -> Result<Uint128, ContractError> {
    Ok(BALANCES
        .may_load(storage, key)
        .unwrap_or(None)
        .unwrap_or_else(Uint128::zero))
}

pub fn save_balance(
    key: AccountStorageKey,
    value: Uint128,
    storage: &mut dyn Storage,
) -> Result<(), ContractError> {
    BALANCES.save(storage, key, &value)?;
    Ok(())
}

pub fn save_withdrawn(
    key: AccountStorageKey,
    value: Uint128,
    storage: &mut dyn Storage,
) -> Result<(), ContractError> {
    WITHDRAWNS.save(storage, key, &value)?;
    Ok(())
}

pub fn load_bond_pledge(
    key: AccountStorageKey,
    storage: &dyn Storage,
) -> Result<Option<PledgeData>, ContractError> {
    Ok(BOND_PLEDGES.may_load(storage, key).unwrap_or(None))
}

pub fn remove_bond_pledge(
    key: AccountStorageKey,
    storage: &mut dyn Storage,
) -> Result<(), ContractError> {
    BOND_PLEDGES.remove(storage, key);
    Ok(())
}

pub fn save_bond_pledge(
    key: AccountStorageKey,
    value: &PledgeData,
    storage: &mut dyn Storage,
) -> Result<(), ContractError> {
    BOND_PLEDGES.save(storage, key, value)?;
    Ok(())
}

pub fn load_gateway_pledge(
    key: AccountStorageKey,
    storage: &dyn Storage,
) -> Result<Option<PledgeData>, ContractError> {
    Ok(GATEWAY_PLEDGES.may_load(storage, key).unwrap_or(None))
}

pub fn save_gateway_pledge(
    key: AccountStorageKey,
    value: &PledgeData,
    storage: &mut dyn Storage,
) -> Result<(), ContractError> {
    GATEWAY_PLEDGES.save(storage, key, value)?;
    Ok(())
}

pub fn remove_gateway_pledge(
    key: AccountStorageKey,
    storage: &mut dyn Storage,
) -> Result<(), ContractError> {
    GATEWAY_PLEDGES.remove(storage, key);
    Ok(())
}

pub fn save_account(account: &Account, storage: &mut dyn Storage) -> Result<(), ContractError> {
    ACCOUNTS.save(storage, account.owner_address(), account)?;
    Ok(())
}

pub fn load_account(
    address: Addr,
    storage: &dyn Storage,
) -> Result<Option<Account>, ContractError> {
    Ok(ACCOUNTS.may_load(storage, address).unwrap_or(None))
}

pub fn delete_account(address: Addr, storage: &mut dyn Storage) -> Result<(), ContractError> {
    ACCOUNTS.remove(storage, address);
    Ok(())
}

fn validate_account(address: Addr, storage: &dyn Storage) -> Result<Account, ContractError> {
    load_account(address.clone(), storage)?
        .ok_or_else(|| ContractError::NoAccountForAddress(address.into_string()))
}

pub fn account_from_address(
    address: &str,
    storage: &dyn Storage,
    api: &dyn Api,
) -> Result<Account, ContractError> {
    validate_account(api.addr_validate(address)?, storage)
}

pub const CONTRACT: Item<ContractVersion> = Item::new("contract_info");

pub fn set_contract_version(store: &mut dyn Storage) -> StdResult<()> {
    let val = ContractVersion {
        contract: "nym-vesting-contract".to_string(),
        version: format!("{MAJOR}.{MINOR}.{PATCH}"),
    };
    CONTRACT.save(store, &val)
}

pub fn get_contract_info(store: &dyn Storage) -> StdResult<ContractVersion> {
    CONTRACT.load(store)
}

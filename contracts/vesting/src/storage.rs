use cosmwasm_std::{Addr, Api, Storage, Uint128};
use cosmwasm_std::{Coin, Order};
use cw_storage_plus::{Item, Map};
use mixnet_contract_common::{IdentityKey, NodeId};
use vesting_contract_common::account::VestingAccountStorageKey;
use vesting_contract_common::{Account, PledgeData, VestingContractError};

pub(crate) type BlockTimestampSecs = u64;

/// Counter for the unique, monotonically increasing storage key id for the vesting account data.
pub const KEY: Item<'_, VestingAccountStorageKey> = Item::new("key");

/// Storage map containing vesting account information associated with particular owner address.
pub const ACCOUNTS: Map<'_, Addr, Account> = Map::new("acc");

/// Storage map containing information about amount of tokens associated with particular vesting account
/// that are currently present in the contract (and have not been withdrawn or staked in the mixnet contract)
// note: this assumes I understood the intent behind this correctly
const BALANCES: Map<'_, VestingAccountStorageKey, Uint128> = Map::new("blc");

/// Storage map containing information about amount of tokens withdrawn from the contract by a particular vesting account.
const WITHDRAWNS: Map<'_, VestingAccountStorageKey, Uint128> = Map::new("wthd");

/// Storage map containing information about amount of tokens pledged towards bonding mixnodes
/// in the mixnet contract using a particular vesting account.
const BOND_PLEDGES: Map<'_, VestingAccountStorageKey, PledgeData> = Map::new("bnd");

/// Storage map containing information about amount of tokens pledged towards bonding gateways
/// in the mixnet contract using a particular vesting account.
const GATEWAY_PLEDGES: Map<'_, VestingAccountStorageKey, PledgeData> = Map::new("gtw");

/// Old, pre-v2 migration, storage map that used to contain information about tokens delegated
/// towards particular mixnodes in the mixnet contract with given vesting account.
/// It should be completely empty.
pub const _OLD_DELEGATIONS: Map<
    '_,
    (VestingAccountStorageKey, IdentityKey, BlockTimestampSecs),
    Uint128,
> = Map::new("dlg");

/// Storage map containing information about tokens delegated towards particular mixnodes
/// in the mixnet contract with given vesting account.
pub const DELEGATIONS: Map<'_, (VestingAccountStorageKey, NodeId, BlockTimestampSecs), Uint128> =
    Map::new("dlg_v2");

/// Explicit contract admin that is allowed, among other things, to create new vesting accounts.
pub const ADMIN: Item<'_, Addr> = Item::new("adm");

/// Address of the mixnet contract.
pub const MIXNET_CONTRACT_ADDRESS: Item<'_, Addr> = Item::new("mix");

/// The denomination of coin used for staking.
pub const MIX_DENOM: Item<'_, String> = Item::new("den");

pub fn save_delegation(
    key: (VestingAccountStorageKey, NodeId, BlockTimestampSecs),
    amount: Uint128,
    storage: &mut dyn Storage,
) -> Result<(), VestingContractError> {
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
    key: (VestingAccountStorageKey, NodeId, BlockTimestampSecs),
    storage: &mut dyn Storage,
) -> Result<(), VestingContractError> {
    DELEGATIONS.remove(storage, key);
    Ok(())
}

pub fn load_delegation_timestamps(
    prefix: (VestingAccountStorageKey, NodeId),
    storage: &dyn Storage,
) -> Result<Vec<BlockTimestampSecs>, VestingContractError> {
    let block_timestamps = DELEGATIONS
        .prefix(prefix)
        .keys(storage, None, None, Order::Ascending)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(block_timestamps)
}

pub fn count_subdelegations_for_mix(
    prefix: (VestingAccountStorageKey, NodeId),
    storage: &dyn Storage,
) -> u32 {
    DELEGATIONS
        .prefix(prefix)
        .keys(storage, None, None, Order::Ascending)
        .count() as u32
}

pub fn load_withdrawn(
    key: VestingAccountStorageKey,
    storage: &dyn Storage,
) -> Result<Uint128, VestingContractError> {
    Ok(WITHDRAWNS
        .may_load(storage, key)
        .unwrap_or(None)
        .unwrap_or_else(Uint128::zero))
}

pub fn load_balance(
    key: VestingAccountStorageKey,
    storage: &dyn Storage,
) -> Result<Uint128, VestingContractError> {
    Ok(BALANCES
        .may_load(storage, key)
        .unwrap_or(None)
        .unwrap_or_else(Uint128::zero))
}

pub fn save_balance(
    key: VestingAccountStorageKey,
    value: Uint128,
    storage: &mut dyn Storage,
) -> Result<(), VestingContractError> {
    BALANCES.save(storage, key, &value)?;
    Ok(())
}

pub fn save_withdrawn(
    key: VestingAccountStorageKey,
    value: Uint128,
    storage: &mut dyn Storage,
) -> Result<(), VestingContractError> {
    WITHDRAWNS.save(storage, key, &value)?;
    Ok(())
}

pub fn load_bond_pledge(
    key: VestingAccountStorageKey,
    storage: &dyn Storage,
) -> Result<Option<PledgeData>, VestingContractError> {
    Ok(BOND_PLEDGES.may_load(storage, key).unwrap_or(None))
}

pub fn remove_bond_pledge(
    key: VestingAccountStorageKey,
    storage: &mut dyn Storage,
) -> Result<(), VestingContractError> {
    BOND_PLEDGES.remove(storage, key);
    Ok(())
}

pub fn save_bond_pledge(
    key: VestingAccountStorageKey,
    value: &PledgeData,
    storage: &mut dyn Storage,
) -> Result<(), VestingContractError> {
    BOND_PLEDGES.save(storage, key, value)?;
    Ok(())
}

pub fn decrease_bond_pledge(
    key: VestingAccountStorageKey,
    amount: Coin,
    storage: &mut dyn Storage,
) -> Result<(), VestingContractError> {
    let mut existing = BOND_PLEDGES.load(storage, key)?;
    if existing.amount.amount <= amount.amount {
        // this shouldn't be possible!
        // (but check for it anyway... just in case)
        return Err(VestingContractError::InvalidBondPledgeReduction {
            current: existing.amount,
            decrease_by: amount,
        });
    }
    existing.amount.amount -= amount.amount;
    save_bond_pledge(key, &existing, storage)
}

pub fn load_gateway_pledge(
    key: VestingAccountStorageKey,
    storage: &dyn Storage,
) -> Result<Option<PledgeData>, VestingContractError> {
    Ok(GATEWAY_PLEDGES.may_load(storage, key).unwrap_or(None))
}

pub fn save_gateway_pledge(
    key: VestingAccountStorageKey,
    value: &PledgeData,
    storage: &mut dyn Storage,
) -> Result<(), VestingContractError> {
    GATEWAY_PLEDGES.save(storage, key, value)?;
    Ok(())
}

pub fn remove_gateway_pledge(
    key: VestingAccountStorageKey,
    storage: &mut dyn Storage,
) -> Result<(), VestingContractError> {
    GATEWAY_PLEDGES.remove(storage, key);
    Ok(())
}

pub fn save_account(
    account: &Account,
    storage: &mut dyn Storage,
) -> Result<(), VestingContractError> {
    ACCOUNTS.save(storage, account.owner_address(), account)?;
    Ok(())
}

pub fn load_account(
    address: Addr,
    storage: &dyn Storage,
) -> Result<Option<Account>, VestingContractError> {
    Ok(ACCOUNTS.may_load(storage, address).unwrap_or(None))
}

pub fn delete_account(
    address: Addr,
    storage: &mut dyn Storage,
) -> Result<(), VestingContractError> {
    ACCOUNTS.remove(storage, address);
    Ok(())
}

fn validate_account(address: Addr, storage: &dyn Storage) -> Result<Account, VestingContractError> {
    load_account(address.clone(), storage)?
        .ok_or_else(|| VestingContractError::NoAccountForAddress(address.into_string()))
}

pub fn account_from_address(
    address: &str,
    storage: &dyn Storage,
    api: &dyn Api,
) -> Result<Account, VestingContractError> {
    validate_account(api.addr_validate(address)?, storage)
}

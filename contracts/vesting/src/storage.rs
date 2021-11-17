// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
use cosmwasm_std::{Addr, StdResult, Storage, Uint128};
use cosmwasm_storage::{bucket, bucket_read, Bucket, ReadonlyBucket};
use mixnet_contract::IdentityKey;
use std::collections::HashMap;

use crate::{
    errors::ContractError,
    vesting::{DelegationData, PeriodicVestingAccount},
};
// storage prefixes
// all of them must be unique and presumably not be a prefix of a different one
// keeping them as short as possible is also desirable as they are part of each stored key
// it's not as important for singletons, but is a nice optimisation for buckets

// buckets
const PREFIX_ACCOUNTS: &[u8] = b"ac";
const PREFIX_ACCOUNT_DELEGATIONS: &[u8] = b"ad";
const PREFIX_ACCOUNT_BALANCE: &[u8] = b"ab";
// Contract-level stuff

fn accounts_mut(storage: &mut dyn Storage) -> Bucket<PeriodicVestingAccount> {
    bucket(storage, PREFIX_ACCOUNTS)
}

fn accounts(storage: &dyn Storage) -> ReadonlyBucket<PeriodicVestingAccount> {
    bucket_read(storage, PREFIX_ACCOUNTS)
}

fn account_delegations_mut(storage: &mut dyn Storage) -> Bucket<Vec<DelegationData>> {
    bucket(storage, PREFIX_ACCOUNT_DELEGATIONS)
}

fn account_delegations(storage: &dyn Storage) -> ReadonlyBucket<Vec<DelegationData>> {
    bucket_read(storage, PREFIX_ACCOUNT_DELEGATIONS)
}

fn account_balance(storage: &dyn Storage) -> ReadonlyBucket<Uint128> {
    bucket_read(storage, PREFIX_ACCOUNT_BALANCE)
}

fn account_balance_mut(storage: &mut dyn Storage) -> Bucket<Uint128> {
    bucket(storage, PREFIX_ACCOUNT_BALANCE)
}

pub fn get_account(storage: &dyn Storage, address: &Addr) -> Option<PeriodicVestingAccount> {
    // Due to using may_load this should be safe to unwrap
    accounts(storage).may_load(address.as_bytes()).unwrap()
}

pub fn set_account(
    storage: &mut dyn Storage,
    account: PeriodicVestingAccount,
) -> Result<(), ContractError> {
    Ok(accounts_mut(storage).save(account.address().as_bytes(), &account)?)
}

pub fn get_account_delegations(
    storage: &dyn Storage,
    address: &Addr,
) -> Option<Vec<DelegationData>> {
    // Due to using may_load this should be safe to unwrap
    account_delegations(storage)
        .may_load(address.as_bytes())
        .unwrap()
}

pub fn set_account_delegations(
    storage: &mut dyn Storage,
    address: &Addr,
    delegations: Vec<DelegationData>,
) -> StdResult<()> {
    account_delegations_mut(storage).save(address.as_bytes(), &delegations)
}

pub fn get_account_balance(storage: &dyn Storage, address: &Addr) -> Option<Uint128> {
    // Due to using may_load this should be safe to unwrap
    account_balance(storage)
        .may_load(address.as_bytes())
        .unwrap()
}

pub fn set_account_balance(
    storage: &mut dyn Storage,
    address: &Addr,
    balance: Uint128,
) -> StdResult<()> {
    account_balance_mut(storage).save(address.as_bytes(), &balance)
}

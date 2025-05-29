// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{CommonStorageKeys, ContractOpts, ContractTester, TestableNymContract};
use cosmwasm_std::testing::message_info;
use cosmwasm_std::{coin, coins, Addr, Coin, MessageInfo};
use rand::RngCore;
use serde::de::DeserializeOwned;
use std::any::type_name;

pub trait StorageReader {
    fn common_key(&self, key: CommonStorageKeys) -> Option<&[u8]>;

    fn read_common_value<T: DeserializeOwned>(&self, key: CommonStorageKeys) -> Option<T> {
        self.read_from_contract_storage(self.common_key(key)?)
    }

    fn unchecked_read_common_value<T: DeserializeOwned>(&self, key: CommonStorageKeys) -> T {
        self.unchecked_read_from_contract_storage(
            self.common_key(key)
                .unwrap_or_else(|| panic!("no key set for {key:?}")),
        )
    }

    fn read_from_contract_storage<T: DeserializeOwned>(&self, key: impl AsRef<[u8]>) -> Option<T>;

    fn unchecked_read_from_contract_storage<T: DeserializeOwned>(
        &self,
        key: impl AsRef<[u8]>,
    ) -> T {
        let typ = type_name::<T>();
        self.read_from_contract_storage(key)
            .unwrap_or_else(|| panic!("value of type {typ} not present in the storage"))
    }
}

// contract that has an admin
pub trait AdminExt: StorageReader {
    fn admin(&self) -> Option<Addr> {
        self.read_common_value(CommonStorageKeys::Admin)
    }

    fn admin_unchecked(&self) -> Addr {
        self.admin().expect("no admin set")
    }

    fn admin_msg(&self) -> MessageInfo {
        message_info(&self.admin_unchecked(), &[])
    }
}

// contract that operates on some specific coin denom
pub trait DenomExt: StorageReader {
    fn denom(&self) -> String {
        self.unchecked_read_common_value(CommonStorageKeys::Denom)
    }

    fn coin(&self, amount: u128) -> Coin {
        coin(amount, self.denom())
    }

    fn coins(&self, amount: u128) -> Vec<Coin> {
        coins(amount, self.denom())
    }
}

pub trait RandExt {
    fn generate_account(&mut self) -> Addr;
}

impl<T> AdminExt for T where T: StorageReader {}
impl<T> DenomExt for T where T: StorageReader {}

impl<C: TestableNymContract> StorageReader for ContractTester<C> {
    fn common_key(&self, key: CommonStorageKeys) -> Option<&[u8]> {
        self.common_storage_keys.get(&key).map(|v| &**v)
    }

    fn read_from_contract_storage<T: DeserializeOwned>(&self, key: impl AsRef<[u8]>) -> Option<T> {
        <Self as ContractOpts>::read_from_contract_storage(self, key)
    }
}

impl<C: TestableNymContract> RandExt for ContractTester<C> {
    fn generate_account(&mut self) -> Addr {
        self.app
            .api()
            .addr_make(&format!("foomp{}", self.rng.next_u64()))
    }
}

// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    CommonStorageKeys, ContractOpts, ContractTester, StorageWrapper, TestableNymContract,
    TEST_DENOM,
};
use cosmwasm_std::testing::message_info;
use cosmwasm_std::{
    coin, coins, from_json, to_json_vec, Addr, Coin, MessageInfo, StdError, StdResult, Storage,
};
use cw_multi_test::Executor;
use cw_storage_plus::{Key, Path, PrimaryKey};
use rand::RngCore;
use rand_chacha::ChaCha20Rng;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::any::type_name;
use std::ops::Deref;

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

// technically it shouldn't rely on `StorageReader` and `common_key` should be extracted
// but this makes it a tad easier and it's only testing code so it's fine
pub trait StorageWriter: StorageReader {
    fn set_common_value<T: Serialize>(
        &mut self,
        key: CommonStorageKeys,
        value: &T,
    ) -> StdResult<()> {
        let key = self
            .common_key(key)
            .ok_or(StdError::not_found("key not found"))?
            .to_vec();
        self.set_storage_value(key, value)
    }

    fn set_storage(&mut self, key: impl AsRef<[u8]>, value: impl AsRef<[u8]>);

    fn set_storage_value<T: Serialize>(
        &mut self,
        key: impl AsRef<[u8]>,
        value: &T,
    ) -> StdResult<()> {
        self.set_storage(key, &to_json_vec(value)?);
        Ok(())
    }
}

pub trait ArbitraryContractStorageReader {
    fn may_read_from_contract_storage(
        &self,
        address: impl Into<String>,
        key: impl AsRef<[u8]>,
    ) -> Option<Vec<u8>>;

    fn must_read_from_contract_storage(
        &self,
        address: impl Into<String>,
        key: impl AsRef<[u8]>,
    ) -> StdResult<Vec<u8>> {
        let key = key.as_ref();
        self.may_read_from_contract_storage(address, key)
            .ok_or(StdError::not_found(format!("no data under {key:?}")))
    }

    fn may_read_value_from_contract_storage<T: DeserializeOwned>(
        &self,
        address: impl Into<String>,
        key: impl AsRef<[u8]>,
    ) -> StdResult<Option<T>> {
        let Some(bytes) = self.may_read_from_contract_storage(address, key) else {
            return Ok(None);
        };

        from_json(&bytes).map(Some)
    }

    fn must_read_value_from_contract_storage<T: DeserializeOwned>(
        &self,
        address: impl Into<String>,
        key: impl AsRef<[u8]>,
    ) -> StdResult<T> {
        let bytes = self.must_read_from_contract_storage(address, key)?;
        from_json(&bytes)
    }
}

pub trait ArbitraryContractStorageWriter {
    fn set_contract_storage(
        &mut self,
        address: impl Into<String>,
        key: impl AsRef<[u8]>,
        value: impl AsRef<[u8]>,
    );

    fn set_contract_storage_value<T: Serialize>(
        &mut self,
        address: impl Into<String>,
        key: impl AsRef<[u8]>,
        value: &T,
    ) -> StdResult<()> {
        self.set_contract_storage(address, key, &to_json_vec(value)?);
        Ok(())
    }

    // attempts to write to an arbitrary contract `cw_storage_plus::Map`
    fn set_contract_map_value<'a, K, T>(
        &mut self,
        address: impl Into<String>,
        namespace: impl AsRef<[u8]>,
        key: K,
        value: &T,
    ) -> StdResult<()>
    where
        K: PrimaryKey<'a>,
        T: Serialize + DeserializeOwned,
    {
        let key_path: Path<T> = Path::new(
            namespace.as_ref(),
            &key.key().iter().map(Key::as_ref).collect::<Vec<_>>(),
        );
        let storage_key = key_path.deref();
        self.set_contract_storage_value(address, storage_key, value)
    }
}

// contract that has an admin
pub trait AdminExt: StorageReader + StorageWriter {
    fn admin(&self) -> Option<Addr> {
        self.read_common_value(CommonStorageKeys::Admin)
    }

    fn update_admin(&mut self, admin: &Option<Addr>) -> StdResult<()> {
        self.set_common_value(CommonStorageKeys::Admin, admin)
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
    fn raw_rng(&mut self) -> &mut ChaCha20Rng;

    fn generate_account(&mut self) -> Addr;

    fn generate_account_with_balance(&mut self) -> Addr
    where
        Self: BankExt;
}

pub trait BankExt {
    fn send_tokens(&mut self, to: Addr, amount: Coin) -> anyhow::Result<()>;
}

impl<T> AdminExt for T where T: StorageReader + StorageWriter {}
impl<T> DenomExt for T where T: StorageReader {}

impl<C: TestableNymContract> StorageReader for ContractTester<C> {
    fn common_key(&self, key: CommonStorageKeys) -> Option<&[u8]> {
        self.common_storage_keys.get(&key).map(|v| &**v)
    }

    fn read_from_contract_storage<T: DeserializeOwned>(&self, key: impl AsRef<[u8]>) -> Option<T> {
        <Self as ContractOpts>::read_from_contract_storage(self, key)
    }
}

impl<C: TestableNymContract> StorageWriter for ContractTester<C> {
    fn set_storage(&mut self, key: impl AsRef<[u8]>, value: impl AsRef<[u8]>) {
        <Self as ContractOpts>::set_contract_storage(self, key, value)
    }
}

impl<C: TestableNymContract> BankExt for ContractTester<C> {
    fn send_tokens(&mut self, to: Addr, amount: Coin) -> anyhow::Result<()> {
        self.app
            .send_tokens(self.master_address.clone(), to, &[amount])?;
        Ok(())
    }
}

impl<C: TestableNymContract> RandExt for ContractTester<C> {
    fn raw_rng(&mut self) -> &mut ChaCha20Rng {
        &mut self.rng
    }

    fn generate_account(&mut self) -> Addr {
        self.app
            .api()
            .addr_make(&format!("foomp{}", self.rng.next_u64()))
    }

    fn generate_account_with_balance(&mut self) -> Addr
    where
        Self: BankExt,
    {
        let addr = self.generate_account();
        let million = 1_000_000_000_000;
        self.send_tokens(addr.clone(), coin(million, TEST_DENOM))
            .unwrap();
        addr
    }
}

impl ArbitraryContractStorageReader for StorageWrapper {
    fn may_read_from_contract_storage(
        &self,
        address: impl Into<String>,
        key: impl AsRef<[u8]>,
    ) -> Option<Vec<u8>> {
        self.contract_storage_wrapper(&Addr::unchecked(address))
            .get(key.as_ref())
    }
}

impl ArbitraryContractStorageWriter for StorageWrapper {
    fn set_contract_storage(
        &mut self,
        address: impl Into<String>,
        key: impl AsRef<[u8]>,
        value: impl AsRef<[u8]>,
    ) {
        // yeah, we're unnecessarily cloning a Rc pointer, but this is a test code, so this inefficiency is fine
        let mut wrapped_storage = self
            .clone()
            .contract_storage_wrapper(&Addr::unchecked(address));
        wrapped_storage.set(key.as_ref(), value.as_ref());
    }
}

impl<C> ArbitraryContractStorageReader for ContractTester<C>
where
    C: TestableNymContract,
{
    fn may_read_from_contract_storage(
        &self,
        address: impl Into<String>,
        key: impl AsRef<[u8]>,
    ) -> Option<Vec<u8>> {
        self.storage
            .as_inner_storage()
            .may_read_from_contract_storage(address, key)
    }
}

impl<C> ArbitraryContractStorageWriter for ContractTester<C>
where
    C: TestableNymContract,
{
    fn set_contract_storage(
        &mut self,
        address: impl Into<String>,
        key: impl AsRef<[u8]>,
        value: impl AsRef<[u8]>,
    ) {
        self.storage
            .as_inner_storage_mut()
            .set_contract_storage(address, key, value);
    }
}

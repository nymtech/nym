// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mock_api::CW12MockApi;
use crate::raw_state::{DecodingError, EncodingError, ImportedContractState, KeyValue};
use crate::AVERAGE_BLOCKTIME_SECS;
use cosmwasm_std::testing::{mock_env, MockQuerier, MockStorage};
use cosmwasm_std::{
    Addr, Coin, Deps, DepsMut, Env, Order, QuerierWrapper, StdResult, Storage, Timestamp,
    TransactionInfo,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

#[cfg(feature = "cw-storage-plus")]
use cw_storage_plus::{Item, Map, PrimaryKey};

// extracted into separate struct for easier cloning, access to mock structs, etc.
// we also had to redefine the MockApi
struct MockedDependencies {
    storage: MockStorage,
    api: CW12MockApi,
    querier: MockQuerier,

    // that's a bit annoying. We have to keep track of all balance changes for when we clone the state
    // as there's no easy way of obtaining the up to date list of all balances from the querier...
    _balances: HashMap<String, Vec<Coin>>,
}

impl MockedDependencies {
    fn new_mock() -> MockedDependencies {
        MockedDependencies {
            storage: Default::default(),
            api: Default::default(),
            querier: Default::default(),
            _balances: Default::default(),
        }
    }

    fn clone_state(&self) -> MockedDependencies {
        let new_querier = MockQuerier::new(
            &self
                ._balances
                .iter()
                .map(|(k, v)| (k.as_ref(), v.as_ref()))
                .collect::<Vec<_>>(),
        );

        let mut new_storage = MockStorage::new();
        for (k, v) in self.storage.range(None, None, Order::Ascending) {
            new_storage.set(&k, &v)
        }

        MockedDependencies {
            storage: new_storage,
            api: self.api,
            querier: new_querier,
            _balances: self._balances.clone(),
        }
    }

    fn from_raw(kvs: Vec<KeyValue>) -> Self {
        let mut new = Self::new_mock();
        for kv in kvs {
            new.storage.set(&kv.key, &kv.value)
        }
        new
    }
}

pub struct ContractState {
    deps: MockedDependencies,
    env: Env,
}

impl ContractState {
    pub fn new() -> Self {
        ContractState {
            deps: MockedDependencies::new_mock(),
            env: mock_env(),
        }
    }

    pub fn new_with_env(env: Env) -> Self {
        ContractState {
            deps: MockedDependencies::new_mock(),
            env,
        }
    }

    pub fn clone_state(&self) -> Self {
        ContractState {
            deps: self.deps.clone_state(),
            env: self.env.clone(),
        }
    }

    // set a new balance for the given address and return the old balance
    pub fn update_account_balance(
        &mut self,
        addr: impl Into<String>,
        balance: Vec<Coin>,
    ) -> Option<Vec<Coin>> {
        // that's a bit annoying. We have to keep track of all balance changes for when we clone the state
        // as there's no easy way of obtaining the up to date list of all balances from the querier...
        let addr = addr.into();
        self.deps._balances.insert(addr.clone(), balance.clone());
        self.deps.querier.update_balance(addr, balance)
    }

    pub fn account_balance(
        &self,
        address: impl Into<String>,
        denom: impl Into<String>,
    ) -> StdResult<Coin> {
        self.deps().querier.query_balance(address, denom)
    }

    pub fn all_account_balances(&self, address: impl Into<String>) -> StdResult<Vec<Coin>> {
        self.deps().querier.query_all_balances(address)
    }

    #[cfg(feature = "cw-storage-plus")]
    pub fn save_map_value<'a, K, T>(&mut self, map: &Map<'a, K, T>, k: K, data: &T) -> StdResult<()>
    where
        T: Serialize + DeserializeOwned,
        K: PrimaryKey<'a>,
    {
        map.save(&mut self.deps.storage, k, data)
    }

    #[cfg(feature = "cw-storage-plus")]
    pub fn load_map_value<'a, K, T>(&self, map: &Map<'a, K, T>, k: K) -> StdResult<T>
    where
        T: Serialize + DeserializeOwned,
        K: PrimaryKey<'a>,
    {
        map.load(&self.deps.storage, k)
    }

    #[cfg(feature = "cw-storage-plus")]
    pub fn may_load_map_value<'a, K, T>(&self, map: &Map<'a, K, T>, k: K) -> StdResult<Option<T>>
    where
        T: Serialize + DeserializeOwned,
        K: PrimaryKey<'a>,
    {
        map.may_load(&self.deps.storage, k)
    }

    #[cfg(feature = "cw-storage-plus")]
    pub fn save_item<T>(&mut self, item: &Item<T>, data: &T) -> StdResult<()>
    where
        T: Serialize + DeserializeOwned,
    {
        item.save(&mut self.deps.storage, data)
    }

    #[cfg(feature = "cw-storage-plus")]
    pub fn load_item<T>(&self, item: &Item<T>) -> StdResult<T>
    where
        T: Serialize + DeserializeOwned,
    {
        item.load(&self.deps.storage)
    }

    #[cfg(feature = "cw-storage-plus")]
    pub fn may_load_item<T>(&self, item: &Item<T>) -> StdResult<Option<T>>
    where
        T: Serialize + DeserializeOwned,
    {
        item.may_load(&self.deps.storage)
    }

    pub fn read_key(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.deps.storage.get(key)
    }

    pub fn set_key_value(&mut self, key: &[u8], value: &[u8]) {
        self.deps.storage.set(key, value)
    }

    pub fn deps(&self) -> Deps<'_> {
        Deps {
            storage: &self.deps.storage,
            api: &self.deps.api,
            querier: QuerierWrapper::new(&self.deps.querier),
        }
    }

    pub fn deps_mut(&mut self) -> DepsMut<'_> {
        DepsMut {
            storage: &mut self.deps.storage,
            api: &self.deps.api,
            querier: QuerierWrapper::new(&self.deps.querier),
        }
    }

    pub fn advance_blocks(&mut self, new_blocks: u64) {
        self.advance_block_height(new_blocks);
        self.advance_blocktime(new_blocks * AVERAGE_BLOCKTIME_SECS)
    }

    pub fn advance_block_height(&mut self, by: u64) {
        self.env.block.height += by;
    }

    pub fn advance_blocktime(&mut self, by_secs: u64) {
        self.env.block.time = self.env.block.time.plus_seconds(by_secs)
    }

    pub fn env(&self) -> &Env {
        &self.env
    }

    pub fn env_cloned(&self) -> Env {
        self.env.clone()
    }

    pub fn contract_address(&self) -> &Addr {
        &self.env.contract.address
    }

    pub fn with_contract_address(mut self, address: impl Into<String>) -> Self {
        self.env.contract.address = Addr::unchecked(address);
        self
    }

    pub fn with_transaction_info(mut self, transaction: Option<TransactionInfo>) -> Self {
        self.env.transaction = transaction;
        self
    }

    pub(crate) fn from_state_dump(state: ImportedContractState, custom_env: Option<Env>) -> Self {
        let env = custom_env.unwrap_or_else(|| {
            // this is not ideal, but we're making an assumption here that block time is approximately 5s
            // at block 5000000, we had a timestamp of 1672411689
            let mut env = mock_env();
            env.block.chain_id = "nyx".to_string();
            env.block.height = state.height;
            if state.height > 5000000 {
                let diff = state.height - 5000000;
                env.block.time =
                    Timestamp::from_seconds(1672411689 + diff * AVERAGE_BLOCKTIME_SECS);
            } else {
                let diff = 5000000 - state.height;
                env.block.time =
                    Timestamp::from_seconds(1672411689 - diff * AVERAGE_BLOCKTIME_SECS);
            }
            env
        });

        let deps = MockedDependencies::from_raw(state.data);

        ContractState { deps, env }
    }

    pub fn try_from_state_dump<P: AsRef<Path>>(
        path: P,
        custom_env: Option<Env>,
    ) -> Result<Self, DecodingError> {
        Ok(ImportedContractState::try_load_from_file(path)?.into_test_mock(custom_env))
    }

    pub fn dump_state<P: AsRef<Path>>(&self, output_path: P) -> Result<(), EncodingError> {
        let mut data = Vec::new();
        for (key, value) in self.deps.storage.range(None, None, Order::Ascending) {
            data.push(KeyValue { key, value })
        }

        let state = ImportedContractState {
            height: self.env.block.height,
            data,
        };

        state.encode().to_file(output_path)
    }
}

impl Default for ContractState {
    fn default() -> Self {
        ContractState {
            deps: MockedDependencies::new_mock(),
            env: mock_env(),
        }
    }
}

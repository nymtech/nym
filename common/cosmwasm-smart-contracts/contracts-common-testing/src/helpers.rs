// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::testing::{message_info, MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{
    coins, Addr, BankMsg, CosmosMsg, Empty, Env, MemoryStorage, MessageInfo, Order, OwnedDeps,
    Response, StdResult, Storage,
};
use cw_storage_plus::{KeyDeserialize, Map, Prefix, PrimaryKey};
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub const TEST_DENOM: &str = "unym";
pub const TEST_PREFIX: &str = "n";

pub fn mock_api() -> MockApi {
    MockApi::default().with_prefix(TEST_PREFIX)
}

pub fn mock_dependencies() -> OwnedDeps<MemoryStorage, MockApi, MockQuerier<Empty>> {
    OwnedDeps {
        storage: MockStorage::default(),
        api: mock_api(),
        querier: MockQuerier::default(),
        custom_query_type: Default::default(),
    }
}

pub fn test_rng() -> ChaCha20Rng {
    let dummy_seed = [42u8; 32];
    rand_chacha::ChaCha20Rng::from_seed(dummy_seed)
}

pub fn deps_with_balance(env: &Env) -> OwnedDeps<MemoryStorage, MockApi, MockQuerier<Empty>> {
    let mut deps = mock_dependencies();
    deps.querier = MockQuerier::<Empty>::new(&[(
        env.contract.address.as_str(),
        coins(100000000000, TEST_DENOM).as_slice(),
    )]);
    deps
}

pub fn generate_sorted_addresses(n: usize) -> Vec<Addr> {
    let mut rng = test_rng();
    let mut addrs = Vec::with_capacity(n);
    for i in 0..n {
        addrs.push(mock_api().addr_make(&format!("addr{i}{}", rng.next_u64())));
    }
    addrs.sort();
    addrs
}

pub fn addr<S: AsRef<str>>(raw: S) -> Addr {
    mock_api().addr_make(raw.as_ref())
}

pub fn sender<S: AsRef<str>>(raw: S) -> MessageInfo {
    message_info(&addr(raw), &[])
}

pub trait ExtractBankMsg {
    fn unwrap_bank_msg(self) -> Option<BankMsg>;
}

impl ExtractBankMsg for Response {
    fn unwrap_bank_msg(self) -> Option<BankMsg> {
        for msg in self.messages {
            match msg.msg {
                CosmosMsg::Bank(bank_msg) => return Some(bank_msg),
                _ => continue,
            }
        }

        None
    }
}

pub trait FullReader<'a> {
    type Key;
    type Value: Serialize + DeserializeOwned;

    fn all_values(&self, store: &dyn Storage) -> StdResult<Vec<Self::Value>>;

    fn all_key_values(&self, store: &dyn Storage) -> StdResult<Vec<(Self::Key, Self::Value)>>;
}

impl<'a, K, T> FullReader<'a> for Map<K, T>
where
    T: Serialize + DeserializeOwned,
    K: PrimaryKey<'a> + KeyDeserialize,
    K::Output: 'static,
{
    type Key = K::Output;
    type Value = T;

    fn all_values(&self, store: &dyn Storage) -> StdResult<Vec<Self::Value>> {
        self.range(store, None, None, Order::Ascending)
            .map(|record| record.map(|r| r.1))
            .collect()
    }

    fn all_key_values(&self, store: &dyn Storage) -> StdResult<Vec<(Self::Key, Self::Value)>> {
        self.range(store, None, None, Order::Ascending).collect()
    }
}

impl<'a, K, T, B> FullReader<'a> for Prefix<K, T, B>
where
    K: KeyDeserialize + 'static,
    T: Serialize + DeserializeOwned,
    B: PrimaryKey<'a>,
{
    type Key = K::Output;
    type Value = T;

    fn all_values(&self, store: &dyn Storage) -> StdResult<Vec<Self::Value>> {
        self.range(store, None, None, Order::Ascending)
            .map(|record| record.map(|r| r.1))
            .collect()
    }

    fn all_key_values(&self, store: &dyn Storage) -> StdResult<Vec<(Self::Key, Self::Value)>> {
        self.range(store, None, None, Order::Ascending).collect()
    }
}

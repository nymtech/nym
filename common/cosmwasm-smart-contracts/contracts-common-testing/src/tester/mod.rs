// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::tester::storage_wrapper::{ContractStorageWrapper, StorageWrapper};
use crate::{mock_api, test_rng, TEST_DENOM};
use cosmwasm_std::testing::MockApi;
use cosmwasm_std::{coin, coins, Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response};
use cw_multi_test::{App, AppBuilder, BankKeeper, Contract, ContractWrapper, Executor};
use rand_chacha::ChaCha20Rng;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::marker::PhantomData;

pub use basic_traits::*;
pub use extensions::*;

mod basic_traits;
mod extensions;
mod storage_wrapper;
// copied from cw-multi-test (but removed generics for custom messages and querier for we don't need them for now)

pub type ContractFn<T, E> =
    fn(deps: DepsMut, env: Env, info: MessageInfo, msg: T) -> Result<Response, E>;
pub type QueryFn<T, E> = fn(deps: Deps, env: Env, msg: T) -> Result<Binary, E>;
pub type PermissionedFn<T, E> = fn(deps: DepsMut, env: Env, msg: T) -> Result<Response, E>;

pub type ContractClosure<T, E> = Box<dyn Fn(DepsMut, Env, MessageInfo, T) -> Result<Response, E>>;
pub type QueryClosure<T, E> = Box<dyn Fn(Deps, Env, T) -> Result<Binary, E>>;

pub trait TestableNymContract {
    const NAME: &'static str;

    type InitMsg: DeserializeOwned + Serialize + Debug + 'static;
    type ExecuteMsg: DeserializeOwned + Serialize + Debug + 'static;
    type QueryMsg: DeserializeOwned + Serialize + Debug + 'static;
    type MigrateMsg: DeserializeOwned + Serialize + Debug + 'static;
    type ContractError: Display + Debug + Send + Sync + 'static;

    fn instantiate() -> ContractFn<Self::InitMsg, Self::ContractError>;
    fn execute() -> ContractFn<Self::ExecuteMsg, Self::ContractError>;
    fn query() -> QueryFn<Self::QueryMsg, Self::ContractError>;
    fn migrate() -> PermissionedFn<Self::MigrateMsg, Self::ContractError>;

    fn base_init_msg() -> Self::InitMsg;

    // // for now we don't care about custom queriers
    // fn contract_wrapper() -> ContractWrapper<
    //     Self::ExecuteMsg,
    //     Self::InitMsg,
    //     Self::QueryMsg,
    //     Self::ContractError,
    //     anyhow::Error,
    //     anyhow::Error,
    //     Empty,
    //     Empty,
    //     Empty,
    //     Self::ContractError,
    //     Self::ContractError,
    //     Self::MigrateMsg,
    //     Self::ContractError,
    // > {
    //     ContractWrapper::new(Self::execute(), Self::instantiate(), Self::query())
    //         .with_migrate(Self::migrate())
    // }

    fn dyn_contract() -> Box<dyn Contract<Empty>> {
        Box::new(
            ContractWrapper::new(Self::execute(), Self::instantiate(), Self::query())
                .with_migrate(Self::migrate()),
        )
    }

    fn init() -> ContractTester<Self>
    where
        Self: Sized,
    {
        let storage = StorageWrapper::new();

        let api = mock_api();
        let master_address = api.addr_make("master-owner");

        let mut app = AppBuilder::new()
            .with_api(api)
            .with_storage(storage.clone())
            .build(|router, _api, storage| {
                router
                    .bank
                    .init_balance(
                        storage,
                        &master_address,
                        coins(1000000000000000, TEST_DENOM),
                    )
                    .unwrap()
            });
        let code_id = app.store_code(Self::dyn_contract());
        let contract_address = app
            .instantiate_contract(
                code_id,
                master_address.clone(),
                &Self::base_init_msg(),
                &[],
                Self::NAME,
                Some(master_address.to_string()),
            )
            .unwrap();

        // send some tokens to the contract
        app.send_tokens(
            master_address.clone(),
            contract_address.clone(),
            &[coin(100000000, TEST_DENOM)],
        )
        .unwrap();

        ContractTester {
            contract: Default::default(),
            app,
            rng: test_rng(),
            storage: storage.contract_storage_wrapper(&contract_address),
            contract_address,
            master_address,
            common_storage_keys: Default::default(),
        }
    }
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum CommonStorageKeys {
    Admin,
    Denom,
}

pub struct ContractTester<C: TestableNymContract> {
    contract: PhantomData<C>,
    pub app: App<BankKeeper, MockApi, StorageWrapper>,
    pub rng: ChaCha20Rng,
    pub contract_address: Addr,
    pub master_address: Addr,
    pub(crate) storage: ContractStorageWrapper,
    pub common_storage_keys: HashMap<CommonStorageKeys, Vec<u8>>,
}

impl<C> ContractTester<C>
where
    C: TestableNymContract,
{
    pub fn insert_common_storage_key(&mut self, key: CommonStorageKeys, value: impl AsRef<[u8]>) {
        self.common_storage_keys
            .insert(key, value.as_ref().to_vec());
    }

    pub fn with_common_storage_key(
        mut self,
        key: CommonStorageKeys,
        value: impl AsRef<[u8]>,
    ) -> Self {
        self.insert_common_storage_key(key, value);
        self
    }
}

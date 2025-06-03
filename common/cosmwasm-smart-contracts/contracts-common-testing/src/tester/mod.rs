// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{mock_api, test_rng, TEST_DENOM};
use cosmwasm_std::testing::MockApi;
use cosmwasm_std::{
    coin, coins, Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, QuerierWrapper, Response,
};
use cw_multi_test::{App, AppBuilder, BankKeeper, Contract, ContractWrapper, Executor};
use rand_chacha::ChaCha20Rng;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::marker::PhantomData;

pub use basic_traits::*;
pub use extensions::*;

pub use crate::tester::storage_wrapper::{ContractStorageWrapper, StorageWrapper};

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
        ContractTesterBuilder::new()
            .instantiate::<Self>(None)
            .build()
    }
}

pub struct ContractTesterBuilder<C> {
    contract: PhantomData<C>,
    master_address: Addr,
    app: App<BankKeeper, MockApi, StorageWrapper>,
    storage: StorageWrapper,
    pub well_known_contracts: HashMap<&'static str, Addr>,
}

impl<C> ContractTesterBuilder<C> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self
    where
        C: TestableNymContract,
    {
        let storage = StorageWrapper::new();

        let api = mock_api();
        let master_address = api.addr_make("master-owner");

        let app = AppBuilder::new()
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

        ContractTesterBuilder {
            contract: Default::default(),
            master_address,
            app,
            storage,
            well_known_contracts: Default::default(),
        }
    }

    pub fn instantiate<D: TestableNymContract>(
        mut self,
        custom_init_msg: Option<D::InitMsg>,
    ) -> ContractTesterBuilder<C> {
        let code_id = self.app.store_code(D::dyn_contract());
        let contract_address = self
            .app
            .instantiate_contract(
                code_id,
                self.master_address.clone(),
                &custom_init_msg.unwrap_or(D::base_init_msg()),
                &[],
                D::NAME,
                Some(self.master_address.to_string()),
            )
            .unwrap();

        // send some tokens to the contract
        self.app
            .send_tokens(
                self.master_address.clone(),
                contract_address.clone(),
                &[coin(100000000, TEST_DENOM)],
            )
            .unwrap();

        self.well_known_contracts.insert(D::NAME, contract_address);
        self
    }

    pub fn build(self) -> ContractTester<C>
    where
        C: TestableNymContract,
    {
        if !self.well_known_contracts.contains_key(C::NAME) {
            panic!("{} contract has not been instantiated", C::NAME);
        }

        let contract_address = self.well_known_contracts[C::NAME].clone();

        ContractTester {
            contract: self.contract,
            app: self.app,
            rng: test_rng(),
            master_address: self.master_address,
            storage: self.storage.contract_storage_wrapper(&contract_address),
            contract_address,
            common_storage_keys: Default::default(),
            well_known_contracts: self.well_known_contracts,
        }
    }

    pub fn contract_storage_wrapper(&self, contract: &Addr) -> ContractStorageWrapper {
        self.storage.contract_storage_wrapper(contract)
    }

    pub fn api(&self) -> MockApi {
        *self.app.api()
    }

    pub fn querier(&self) -> QuerierWrapper {
        self.app.wrap()
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

    // TODO: limitation: doesn't allow multiple contracts of the same type (but that's fine for the time being)
    pub well_known_contracts: HashMap<&'static str, Addr>,
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

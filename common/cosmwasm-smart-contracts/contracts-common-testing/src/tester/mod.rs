// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::tester::storage_wrapper::{ContractStorageWrapper, StorageWrapper};
use crate::{test_rng, TEST_DENOM};
use cosmwasm_std::testing::{message_info, mock_env, MockApi};
use cosmwasm_std::{
    coin, coins, from_json, Addr, Binary, Coin, ContractInfo, Deps, DepsMut, Empty, Env,
    MessageInfo, Response, StdResult, Storage,
};
use cw_multi_test::{
    next_block, App, AppBuilder, AppResponse, BankKeeper, Contract, ContractWrapper, Executor,
};
use rand::RngCore;
use rand_chacha::ChaCha20Rng;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::any::type_name;
use std::fmt::{Debug, Display};
use std::marker::PhantomData;

pub use extensions::*;

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

        let api = MockApi::default().with_prefix("n");
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
        }
    }
}

pub struct ContractTester<C: TestableNymContract> {
    contract: PhantomData<C>,
    pub app: App<BankKeeper, MockApi, StorageWrapper>,
    pub rng: ChaCha20Rng,
    pub contract_address: Addr,
    pub master_address: Addr,
    pub(crate) storage: ContractStorageWrapper,
}

impl<C> ContractTester<C>
where
    C: TestableNymContract,
{
    pub fn set_contract_balance(&mut self, balance: Coin) {
        let contract_address = &self.contract_address;
        self.app
            .router()
            .bank
            .init_balance(
                &mut self.storage.inner_storage(),
                contract_address,
                vec![balance],
            )
            .unwrap();
    }

    pub fn deps(&self) -> Deps<'_> {
        Deps {
            storage: &self.storage,
            api: self.app.api(),
            querier: self.app.wrap(),
        }
    }

    pub fn deps_mut(&mut self) -> DepsMut<'_> {
        DepsMut {
            storage: &mut self.storage,
            api: self.app.api(),
            querier: self.app.wrap(),
        }
    }

    pub fn deps_mut_env(&mut self) -> (DepsMut<'_>, Env) {
        let env = self.env().clone();
        (self.deps_mut(), env)
    }

    pub fn storage(&self) -> &dyn Storage {
        &self.storage
    }

    pub fn storage_mut(&mut self) -> &mut dyn Storage {
        &mut self.storage
    }

    pub fn env(&self) -> Env {
        Env {
            block: self.app.block_info(),
            contract: ContractInfo {
                address: self.contract_address.clone(),
            },
            ..mock_env()
        }
    }

    pub fn next_block(&mut self) {
        self.app.update_block(next_block)
    }

    pub fn read_from_contract_storage<T: DeserializeOwned>(
        &self,
        key: impl AsRef<[u8]>,
    ) -> Option<T> {
        let raw = self.deps().storage.get(key.as_ref())?;
        from_json(&raw).ok()
    }

    pub fn unchecked_read_from_contract_storage<T: DeserializeOwned>(
        &self,
        key: impl AsRef<[u8]>,
    ) -> T {
        let typ = type_name::<T>();
        self.read_from_contract_storage(key)
            .unwrap_or_else(|| panic!("value of type {typ} not present in the storage"))
    }

    pub fn execute_raw(
        &mut self,
        sender: Addr,
        message: C::ExecuteMsg,
    ) -> Result<Response, C::ContractError> {
        self.execute_raw_with_balance(sender, &[], message)
    }

    pub fn execute_raw_with_balance(
        &mut self,
        sender: Addr,
        coins: &[Coin],
        message: C::ExecuteMsg,
    ) -> Result<Response, C::ContractError> {
        let env = self.env();
        let info = message_info(&sender, coins);

        C::execute()(self.deps_mut(), env, info, message)
    }

    pub fn execute_msg(
        &mut self,
        sender: Addr,
        message: &C::ExecuteMsg,
    ) -> anyhow::Result<AppResponse> {
        self.execute_msg_with_balance(sender, &[], message)
    }

    pub fn execute_msg_with_balance(
        &mut self,
        sender: Addr,
        coins: &[Coin],
        message: &C::ExecuteMsg,
    ) -> anyhow::Result<AppResponse> {
        self.app
            .execute_contract(sender, self.contract_address.clone(), message, coins)
    }

    pub fn query<T: DeserializeOwned>(&self, message: &C::QueryMsg) -> StdResult<T> {
        self.app
            .wrap()
            .query_wasm_smart(self.contract_address.as_str(), message)
    }

    pub fn generate_account(&mut self) -> Addr {
        self.app
            .api()
            .addr_make(&format!("foomp{}", self.rng.next_u64()))
    }
}

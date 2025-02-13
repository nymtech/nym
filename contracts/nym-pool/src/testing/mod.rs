// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract;
use crate::contract::{execute, instantiate, migrate, query};
use crate::storage::NYM_POOL_STORAGE;
use crate::testing::storage::{ContractStorageWrapper, StorageWrapper};
use cosmwasm_std::testing::{message_info, mock_env, MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{
    coin, coins, Addr, Coin, ContractInfo, Deps, DepsMut, Empty, Env, MemoryStorage, MessageInfo,
    Order, OwnedDeps, Response, StdResult, Storage, Uint128,
};
use cw_multi_test::{
    next_block, App, AppBuilder, AppResponse, BankKeeper, Contract, ContractWrapper, Executor,
};
use nym_pool_contract_common::{
    Allowance, BasicAllowance, ExecuteMsg, Grant, InstantiateMsg, NymPoolContractError, QueryMsg,
};
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use serde::de::DeserializeOwned;
use std::collections::HashMap;

mod storage;

pub fn test_rng() -> ChaCha20Rng {
    let dummy_seed = [42u8; 32];
    ChaCha20Rng::from_seed(dummy_seed)
}

pub fn deps_with_balance(env: &Env) -> OwnedDeps<MemoryStorage, MockApi, MockQuerier<Empty>> {
    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: MockQuerier::<Empty>::new(&[(
            env.contract.address.as_str(),
            coins(100000000000, TEST_DENOM).as_slice(),
        )]),
        custom_query_type: Default::default(),
    }
}

pub const TEST_DENOM: &str = "unym";

pub struct TestSetup {
    pub app: App<BankKeeper, MockApi, StorageWrapper>,
    pub rng: ChaCha20Rng,
    pub contract_address: Addr,
    pub master_address: Addr,
    pub(crate) storage: ContractStorageWrapper,
}

pub fn contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(execute, instantiate, query).with_migrate(migrate);
    Box::new(contract)
}

impl TestSetup {
    pub fn init() -> TestSetup {
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
        let code_id = app.store_code(contract());
        let contract_address = app
            .instantiate_contract(
                code_id,
                master_address.clone(),
                &InstantiateMsg {
                    pool_denomination: TEST_DENOM.to_string(),
                    grants: Default::default(),
                },
                &[],
                "nym-pool-contract",
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

        TestSetup {
            app,
            rng: test_rng(),
            storage: storage.contract_storage_wrapper(&contract_address),
            contract_address,
            master_address,
        }
    }

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

    pub fn execute_raw(
        &mut self,
        sender: Addr,
        message: ExecuteMsg,
    ) -> Result<Response, NymPoolContractError> {
        self.execute_raw_with_balance(sender, &[], message)
    }

    pub fn execute_raw_with_balance(
        &mut self,
        sender: Addr,
        coins: &[Coin],
        message: ExecuteMsg,
    ) -> Result<Response, NymPoolContractError> {
        let env = self.env();
        let info = message_info(&sender, coins);
        contract::execute(self.deps_mut(), env, info, message)
    }

    pub fn execute_msg(
        &mut self,
        sender: Addr,
        message: &ExecuteMsg,
    ) -> anyhow::Result<AppResponse> {
        self.execute_msg_with_balance(sender, &[], message)
    }

    pub fn execute_msg_with_balance(
        &mut self,
        sender: Addr,
        coins: &[Coin],
        message: &ExecuteMsg,
    ) -> anyhow::Result<AppResponse> {
        self.app
            .execute_contract(sender, self.contract_address.clone(), message, coins)
    }

    pub fn query<T: DeserializeOwned>(&self, message: &QueryMsg) -> StdResult<T> {
        self.app
            .wrap()
            .query_wasm_smart(self.contract_address.as_str(), message)
    }

    pub fn generate_account(&mut self) -> Addr {
        self.app
            .api()
            .addr_make(&format!("foomp{}", self.rng.next_u64()))
    }

    pub fn admin_unchecked(&self) -> Addr {
        NYM_POOL_STORAGE
            .contract_admin
            .get(self.deps())
            .unwrap()
            .unwrap()
    }

    pub fn change_admin(&mut self, new_admin: &Addr) {
        self.execute_msg(
            self.admin_unchecked(),
            &ExecuteMsg::UpdateAdmin {
                admin: new_admin.to_string(),
                update_granter_set: Some(true),
            },
        )
        .unwrap();
    }

    pub fn admin_msg(&self) -> MessageInfo {
        message_info(&self.admin_unchecked(), &[])
    }

    pub fn denom(&self) -> String {
        NYM_POOL_STORAGE
            .pool_denomination
            .load(self.storage())
            .unwrap()
    }

    pub fn coin(&self, amount: u128) -> Coin {
        coin(amount, self.denom())
    }

    pub fn coins(&self, amount: u128) -> Vec<Coin> {
        coins(amount, self.denom())
    }

    #[track_caller]
    pub fn add_dummy_grant(&mut self) -> Grant {
        let grantee = self.generate_account();
        self.add_dummy_grant_for(&grantee)
    }

    #[track_caller]
    pub fn add_dummy_grant_for(&mut self, grantee: impl Into<String>) -> Grant {
        let grantee = Addr::unchecked(grantee);
        let granter = self.admin_unchecked();
        let env = self.env();
        NYM_POOL_STORAGE
            .insert_new_grant(
                self.deps_mut(),
                &env,
                &granter,
                &grantee,
                Allowance::Basic(BasicAllowance::unlimited()),
            )
            .unwrap();

        NYM_POOL_STORAGE.load_grant(self.deps(), &grantee).unwrap()
    }

    #[track_caller]
    pub fn lock_allowance(&mut self, grantee: impl Into<String>, amount: impl Into<Uint128>) {
        let denom = NYM_POOL_STORAGE
            .pool_denomination
            .load(self.deps().storage)
            .unwrap();

        self.execute_msg(
            Addr::unchecked(grantee),
            &ExecuteMsg::LockAllowance {
                amount: coin(amount.into().u128(), denom),
            },
        )
        .unwrap();
    }

    #[track_caller]
    pub fn full_locked_map(&self) -> HashMap<Addr, Uint128> {
        NYM_POOL_STORAGE
            .locked
            .grantees
            .range(self.deps().storage, None, None, Order::Ascending)
            .collect::<Result<HashMap<_, _>, _>>()
            .unwrap()
    }

    #[track_caller]
    pub fn add_granter(&mut self, granter: &Addr) {
        let env = self.env();
        let admin = self.admin_unchecked();
        NYM_POOL_STORAGE
            .add_new_granter(self.deps_mut(), &env, &admin, granter)
            .unwrap();
    }
}

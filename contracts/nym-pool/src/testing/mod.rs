// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract::{execute, instantiate, migrate, query};
use cosmwasm_std::testing::{mock_env, MockApi, MockStorage};
use cosmwasm_std::{
    coins, Addr, ContractInfo, Deps, DepsMut, Empty, Env, MemoryStorage, Order, Record, Storage,
};
use cw_multi_test::{App, AppBuilder, BankKeeper, Contract, ContractWrapper, Executor};
use nym_pool_contract_common::InstantiateMsg;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct StorageWrapper(Rc<RefCell<MemoryStorage>>);

impl Storage for StorageWrapper {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.0.borrow().get(key)
    }

    fn set(&mut self, key: &[u8], value: &[u8]) {
        self.0.borrow_mut().set(key, value);
    }

    fn remove(&mut self, key: &[u8]) {
        self.0.borrow_mut().remove(key);
    }

    fn range<'a>(
        &'a self,
        start: Option<&[u8]>,
        end: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        todo!()
    }
}

pub fn test_rng() -> ChaCha20Rng {
    let dummy_seed = [42u8; 32];
    ChaCha20Rng::from_seed(dummy_seed)
}

pub const TEST_DENOM: &str = "unym";

pub struct TestSetup {
    pub app: App<BankKeeper, MockApi, StorageWrapper>,
    pub rng: ChaCha20Rng,
    pub contract_address: Addr,
    pub master_address: Addr,
    pub storage: StorageWrapper,
}

pub fn contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(execute, instantiate, query).with_migrate(migrate);
    Box::new(contract)
}

impl TestSetup {
    pub fn init() -> TestSetup {
        let storage = StorageWrapper(Rc::new(RefCell::new(MockStorage::new())));

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
                },
                &[],
                "nym-pool-contract",
                Some(master_address.to_string()),
            )
            .unwrap();

        TestSetup {
            app,
            rng: test_rng(),
            contract_address,
            master_address,
            storage,
        }
    }

    pub fn deps(&self) -> Deps<'_> {
        Deps {
            storage: self.app.storage(),
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

    pub fn env(&self) -> Env {
        Env {
            block: self.app.block_info(),
            contract: ContractInfo {
                address: self.contract_address.clone(),
            },
            ..mock_env()
        }
    }
}

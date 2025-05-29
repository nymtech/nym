// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{ContractTester, TestableNymContract};
use cosmwasm_std::testing::{message_info, mock_env};
use cosmwasm_std::{
    from_json, Addr, Coin, ContractInfo, Deps, DepsMut, Env, Response, StdResult, Storage,
};
use cw_multi_test::{next_block, AppResponse, Executor};
use serde::de::DeserializeOwned;
use std::any::type_name;

pub trait ContractOpts {
    type ExecuteMsg;
    type QueryMsg;
    type ContractError;

    fn deps(&self) -> Deps<'_>;

    fn deps_mut(&mut self) -> DepsMut<'_>;

    fn env(&self) -> Env;

    fn deps_mut_env(&mut self) -> (DepsMut<'_>, Env) {
        let env = self.env().clone();
        (self.deps_mut(), env)
    }

    fn storage(&self) -> &dyn Storage;

    fn storage_mut(&mut self) -> &mut dyn Storage;

    fn read_from_contract_storage<T: DeserializeOwned>(&self, key: impl AsRef<[u8]>) -> Option<T>;

    fn unchecked_read_from_contract_storage<T: DeserializeOwned>(
        &self,
        key: impl AsRef<[u8]>,
    ) -> T {
        let typ = type_name::<T>();
        self.read_from_contract_storage(key)
            .unwrap_or_else(|| panic!("value of type {typ} not present in the storage"))
    }

    fn execute_raw(
        &mut self,
        sender: Addr,
        message: Self::ExecuteMsg,
    ) -> Result<Response, Self::ContractError> {
        self.execute_raw_with_balance(sender, &[], message)
    }

    fn execute_raw_with_balance(
        &mut self,
        sender: Addr,
        coins: &[Coin],
        message: Self::ExecuteMsg,
    ) -> Result<Response, Self::ContractError>;
}

impl<C> ContractOpts for ContractTester<C>
where
    C: TestableNymContract,
{
    type ExecuteMsg = C::ExecuteMsg;
    type QueryMsg = C::QueryMsg;
    type ContractError = C::ContractError;

    fn deps(&self) -> Deps<'_> {
        Deps {
            storage: &self.storage,
            api: self.app.api(),
            querier: self.app.wrap(),
        }
    }

    fn deps_mut(&mut self) -> DepsMut<'_> {
        DepsMut {
            storage: &mut self.storage,
            api: self.app.api(),
            querier: self.app.wrap(),
        }
    }

    fn env(&self) -> Env {
        Env {
            block: self.app.block_info(),
            contract: ContractInfo {
                address: self.contract_address.clone(),
            },
            ..mock_env()
        }
    }

    fn storage(&self) -> &dyn Storage {
        &self.storage
    }

    fn storage_mut(&mut self) -> &mut dyn Storage {
        &mut self.storage
    }

    fn read_from_contract_storage<T: DeserializeOwned>(&self, key: impl AsRef<[u8]>) -> Option<T> {
        let raw = self.deps().storage.get(key.as_ref())?;
        from_json(&raw).ok()
    }

    fn execute_raw_with_balance(
        &mut self,
        sender: Addr,
        coins: &[Coin],
        message: C::ExecuteMsg,
    ) -> Result<Response, C::ContractError> {
        let env = self.env();
        let info = message_info(&sender, coins);

        C::execute()(self.deps_mut(), env, info, message)
    }
}

pub trait ChainOpts: ContractOpts {
    fn set_contract_balance(&mut self, balance: Coin);

    fn next_block(&mut self);

    fn execute_msg(
        &mut self,
        sender: Addr,
        message: &Self::ExecuteMsg,
    ) -> anyhow::Result<AppResponse> {
        self.execute_msg_with_balance(sender, &[], message)
    }

    fn execute_msg_with_balance(
        &mut self,
        sender: Addr,
        coins: &[Coin],
        message: &Self::ExecuteMsg,
    ) -> anyhow::Result<AppResponse>;

    fn query<T: DeserializeOwned>(&self, message: &Self::QueryMsg) -> StdResult<T>;
}

impl<C> ChainOpts for ContractTester<C>
where
    C: TestableNymContract,
{
    fn set_contract_balance(&mut self, balance: Coin) {
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
    fn next_block(&mut self) {
        self.app.update_block(next_block)
    }

    fn execute_msg(
        &mut self,
        sender: Addr,
        message: &C::ExecuteMsg,
    ) -> anyhow::Result<AppResponse> {
        self.execute_msg_with_balance(sender, &[], message)
    }

    fn execute_msg_with_balance(
        &mut self,
        sender: Addr,
        coins: &[Coin],
        message: &C::ExecuteMsg,
    ) -> anyhow::Result<AppResponse> {
        self.app
            .execute_contract(sender, self.contract_address.clone(), message, coins)
    }

    fn query<T: DeserializeOwned>(&self, message: &C::QueryMsg) -> StdResult<T> {
        self.app
            .wrap()
            .query_wasm_smart(self.contract_address.as_str(), message)
    }
}

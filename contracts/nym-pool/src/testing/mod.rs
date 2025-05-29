// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract::{execute, instantiate, migrate, query};
use crate::storage::NYM_POOL_STORAGE;
use cosmwasm_std::{Addr, Order, Uint128};
use nym_contracts_common_testing::{
    AdminExt, ChainOpts, CommonStorageKeys, ContractFn, ContractOpts, ContractTester, DenomExt,
    PermissionedFn, QueryFn, RandExt, TestableNymContract,
};
use nym_pool_contract_common::constants::storage_keys;
use nym_pool_contract_common::{
    Allowance, BasicAllowance, ExecuteMsg, Grant, InstantiateMsg, MigrateMsg, NymPoolContractError,
    QueryMsg,
};
use std::collections::HashMap;

pub use nym_contracts_common_testing::TEST_DENOM;

pub struct NymPoolContract;

impl TestableNymContract for NymPoolContract {
    const NAME: &'static str = "nym-pool-contract";
    type InitMsg = InstantiateMsg;
    type ExecuteMsg = ExecuteMsg;
    type QueryMsg = QueryMsg;
    type MigrateMsg = MigrateMsg;
    type ContractError = NymPoolContractError;

    fn instantiate() -> ContractFn<Self::InitMsg, Self::ContractError> {
        instantiate
    }

    fn execute() -> ContractFn<Self::ExecuteMsg, Self::ContractError> {
        execute
    }

    fn query() -> QueryFn<Self::QueryMsg, Self::ContractError> {
        query
    }

    fn migrate() -> PermissionedFn<Self::MigrateMsg, Self::ContractError> {
        migrate
    }

    fn base_init_msg() -> Self::InitMsg {
        InstantiateMsg {
            pool_denomination: TEST_DENOM.to_string(),
            grants: Default::default(),
        }
    }
}

pub fn init_contract_tester() -> ContractTester<NymPoolContract> {
    NymPoolContract::init()
        .with_common_storage_key(CommonStorageKeys::Admin, storage_keys::CONTRACT_ADMIN)
        .with_common_storage_key(CommonStorageKeys::Denom, storage_keys::POOL_DENOMINATION)
}

pub trait NymPoolContractTesterExt:
    ContractOpts<ExecuteMsg = ExecuteMsg, QueryMsg = QueryMsg, ContractError = NymPoolContractError>
    + ChainOpts
    + AdminExt
    + DenomExt
    + RandExt
{
    fn change_admin(&mut self, new_admin: &Addr) {
        self.execute_msg(
            self.admin_unchecked(),
            &ExecuteMsg::UpdateAdmin {
                admin: new_admin.to_string(),
                update_granter_set: Some(true),
            },
        )
        .unwrap();
    }

    #[track_caller]
    fn add_dummy_grant(&mut self) -> Grant {
        let grantee = self.generate_account();
        self.add_dummy_grant_for(&grantee)
    }

    #[track_caller]
    fn add_dummy_grant_for(&mut self, grantee: impl Into<String>) -> Grant {
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
    fn lock_allowance(&mut self, grantee: impl Into<String>, amount: impl Into<Uint128>) {
        self.execute_msg(
            Addr::unchecked(grantee),
            &ExecuteMsg::LockAllowance {
                amount: self.coin(amount.into().u128()),
            },
        )
        .unwrap();
    }

    #[track_caller]
    fn full_locked_map(&self) -> HashMap<Addr, Uint128> {
        NYM_POOL_STORAGE
            .locked
            .grantees
            .range(self.deps().storage, None, None, Order::Ascending)
            .collect::<Result<HashMap<_, _>, _>>()
            .unwrap()
    }

    #[track_caller]
    fn add_granter(&mut self, granter: &Addr) {
        let env = self.env();
        let admin = self.admin_unchecked();
        NYM_POOL_STORAGE
            .add_new_granter(self.deps_mut(), &env, &admin, granter)
            .unwrap();
    }
}

impl NymPoolContractTesterExt for ContractTester<NymPoolContract> {}

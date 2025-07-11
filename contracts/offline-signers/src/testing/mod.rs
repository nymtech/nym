// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract::{execute, instantiate, migrate, query};
use nym_contracts_common_testing::{
    AdminExt, ArbitraryContractStorageReader, ArbitraryContractStorageWriter, BankExt, ChainOpts,
    CommonStorageKeys, ContractFn, ContractOpts, ContractTester, DenomExt, PermissionedFn, QueryFn,
    RandExt, TestableNymContract,
};
use nym_offline_signers_common::constants::storage_keys;
use nym_offline_signers_common::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, NymOfflineSignersContractError, QueryMsg,
};

pub struct OfflineSignersContract;

impl TestableNymContract for OfflineSignersContract {
    const NAME: &'static str = "offline-signers-contract";
    type InitMsg = InstantiateMsg;
    type ExecuteMsg = ExecuteMsg;
    type QueryMsg = QueryMsg;
    type MigrateMsg = MigrateMsg;
    type ContractError = NymOfflineSignersContractError;

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
        InstantiateMsg {}
    }
}

pub fn init_contract_tester() -> ContractTester<OfflineSignersContract> {
    OfflineSignersContract::init()
        .with_common_storage_key(CommonStorageKeys::Admin, storage_keys::CONTRACT_ADMIN)
}

#[allow(dead_code)]
pub(crate) trait OfflineSignersContractTesterExt:
    ContractOpts<
        ExecuteMsg = ExecuteMsg,
        QueryMsg = QueryMsg,
        ContractError = NymOfflineSignersContractError,
    > + ChainOpts
    + AdminExt
    + DenomExt
    + RandExt
    + BankExt
    + ArbitraryContractStorageReader
    + ArbitraryContractStorageWriter
{
}

impl OfflineSignersContractTesterExt for ContractTester<OfflineSignersContract> {}

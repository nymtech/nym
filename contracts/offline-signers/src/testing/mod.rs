// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract::{execute, instantiate, migrate, query};
use nym_coconut_dkg::testable_dkg_contract::DkgContract;
use nym_contracts_common_testing::{
    AdminExt, ArbitraryContractStorageReader, ArbitraryContractStorageWriter, BankExt, ChainOpts,
    CommonStorageKeys, ContractFn, ContractOpts, ContractTester, DenomExt, PermissionedFn, QueryFn,
    RandExt, TestableNymContract,
};
use nym_offline_signers_contract_common::constants::storage_keys;
use nym_offline_signers_contract_common::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, NymOfflineSignersContractError, QueryMsg,
};

pub struct OfflineSignersContract;

const DEFAULT_GROUP_MEMBERS: usize = 15;

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

    fn init() -> ContractTester<Self>
    where
        Self: Sized,
    {
        init_contract_tester_with_group_members(DEFAULT_GROUP_MEMBERS)
    }
}

pub fn init_contract_tester() -> ContractTester<OfflineSignersContract> {
    OfflineSignersContract::init()
        .with_common_storage_key(CommonStorageKeys::Admin, storage_keys::CONTRACT_ADMIN)
}

pub fn init_contract_tester_with_group_members(
    members: usize,
) -> ContractTester<OfflineSignersContract> {
    // prepare the dkg contract and using that initial setup, add the offline signers contract
    let builder =
        nym_coconut_dkg::testable_dkg_contract::prepare_contract_tester_builder_with_group_members(
            members,
        );

    // we just instantiated it
    let dkg_contract_address = builder.unchecked_contract_address::<DkgContract>();

    // 5. finally init the offline signers contract
    builder
        .instantiate::<OfflineSignersContract>(Some(InstantiateMsg {
            dkg_contract_address: dkg_contract_address.to_string(),
        }))
        .build()
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

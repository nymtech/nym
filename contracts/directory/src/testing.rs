// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// fine in test code
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use crate::contract::{execute, instantiate, migrate, query};
use cosmwasm_std::Storage;
use mixnet_contract::testable_mixnet_contract::{EmbeddedMixnetContractExt, MixnetContract};
use nym_contracts_common_testing::{
    AdminExt, ArbitraryContractStorageReader, ArbitraryContractStorageWriter, BankExt, ChainOpts,
    CommonStorageKeys, ContractFn, ContractOpts, ContractTester, ContractTesterBuilder, DenomExt,
    PermissionedFn, QueryFn, RandExt, TestableNymContract,
};
use nym_directory_contract_common::constants::storage_keys;
use nym_directory_contract_common::{
    DirectoryContractError, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};

pub struct DirectoryContract;

impl TestableNymContract for DirectoryContract {
    const NAME: &'static str = "directory-contract";
    type InitMsg = InstantiateMsg;
    type ExecuteMsg = ExecuteMsg;
    type QueryMsg = QueryMsg;
    type MigrateMsg = MigrateMsg;
    type ContractError = DirectoryContractError;

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
        let builder = ContractTesterBuilder::new().instantiate::<MixnetContract>(None);

        // we just instantiated it
        let mixnet_address = builder
            .well_known_contracts
            .get(MixnetContract::NAME)
            .unwrap()
            .clone();

        builder
            .instantiate::<Self>(Some(InstantiateMsg {
                mixnet_contract_address: mixnet_address.to_string(),
                initial_labels: vec![],
            }))
            .build()
    }
}

/// Storage key the mixnet contract uses for its `ContractState` `Item`
/// (mirrors `mixnet/src/constants.rs::CONTRACT_STATE_KEY`).
const MIXNET_CONTRACT_STATE_STORAGE_KEY: &str = "state";

pub fn init_contract_tester() -> ContractTester<DirectoryContract> {
    let mut tester = DirectoryContract::init()
        .with_common_storage_key(CommonStorageKeys::Admin, storage_keys::CONTRACT_ADMIN);

    // TODO: not implemented yet

    // // Chicken-and-egg: the mixnet contract is instantiated first and is given
    // // a placeholder `directory_contract_address` because the directory
    // // contract doesn't exist yet. Once the directory contract has been
    // // instantiated we patch the mixnet's stored `ContractState` so that the
    // // unbond callback (`OnNymNodeUnbond`) actually dispatches to the right
    // // contract. In production this fixup happens via a contract migration;
    // // here we go straight to storage to avoid jumping through cw2 version
    // // checks that don't apply on a fresh tester.
    // let directory_address = tester.contract_address.clone();
    // let mut mixnet_state: ContractState = tester
    //     .read_from_mixnet_contract_storage(MIXNET_CONTRACT_STATE_STORAGE_KEY)
    //     .expect("mixnet contract state should be loadable");
    // mixnet_state.directory_contract_address = directory_address;
    // tester
    //     .write_to_mixnet_contract_storage_value(MIXNET_CONTRACT_STATE_STORAGE_KEY, &mixnet_state)
    //     .expect("should be able to patch mixnet contract state");

    tester
}

pub trait DirectoryContractTesterExt:
    ContractOpts<
        ExecuteMsg = ExecuteMsg,
        QueryMsg = QueryMsg,
        ContractError = DirectoryContractError,
    > + ChainOpts
    + AdminExt
    + DenomExt
    + BankExt
    + RandExt
    + Storage
    + ArbitraryContractStorageReader
    + ArbitraryContractStorageWriter
    + EmbeddedMixnetContractExt
    + Sized
{
}

impl DirectoryContractTesterExt for ContractTester<DirectoryContract> {}

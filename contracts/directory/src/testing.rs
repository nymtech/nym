// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// fine in test code
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use crate::contract::{execute, instantiate, migrate, query};
use cosmwasm_std::{Binary, Storage};
use mixnet_contract::testable_mixnet_contract::{
    EmbeddedMixnetContractExt, MixnetContract, MixnetContractSiblings,
};
use nym_contracts_common_testing::{
    AdminExt, ArbitraryContractStorageReader, ArbitraryContractStorageWriter, BankExt, ChainOpts,
    CommonStorageKeys, ContractFn, ContractOpts, ContractTester, ContractTesterBuilder, DenomExt,
    PermissionedFn, QueryFn, RandExt, TestableNymContract,
};
use nym_crypto::asymmetric::ed25519;
use nym_directory_contract_common::constants::storage_keys;
use nym_directory_contract_common::{
    node_signing_payload, DirectoryContractError, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};
use nym_mixnet_contract_common::NodeId;

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

pub fn init_contract_tester() -> ContractTester<DirectoryContract> {
    let mut tester = DirectoryContract::init()
        .with_common_storage_key(CommonStorageKeys::Admin, storage_keys::CONTRACT_ADMIN);

    let directory_address = tester.contract_address.clone();
    tester
        .set_mixnet_sibling_contracts(
            MixnetContractSiblings::default()
                .with_clear_all()
                .with_directory_contract(directory_address),
        )
        .expect("should be able to patch mixnet contract state");

    tester
}

/// Sign a node-entry write/delete payload with the node's ed25519 identity key,
/// producing the `signature` a `SetNodeEntry`/`DeleteNodeEntry` message carries.
/// Pair with [`EmbeddedMixnetContractExt::bond_dummy_nymnode_with_keypair`], whose
/// returned keypair matches the bonded node's on-chain identity key. A delete signs
/// the canonical payload with empty `data`.
pub fn sign_node_payload(
    keypair: &ed25519::KeyPair,
    node_id: NodeId,
    label: &str,
    sequence: u64,
    data: &[u8],
) -> Binary {
    let payload = node_signing_payload(node_id, label, sequence, data);
    Binary::from(keypair.private_key().sign(payload).to_bytes().as_ref())
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

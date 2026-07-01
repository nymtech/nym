// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// fine in test code
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use crate::contract::{execute, instantiate, migrate, query};
use crate::storage::NYM_DIRECTORY_CONTRACT_STORAGE;
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
    node_signing_payload, CuratedEntry, DirectoryContractError, ExecuteMsg, InstantiateMsg,
    MigrateMsg, NodeEntry, QueryMsg,
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

/// Like [`init_contract_tester`] but also deploys a real node-families contract and
/// points the mixnet at both. The mixnet unbond handler fires HARD sub-messages to
/// node-families AND the directory, so both must be dispatchable for a real
/// `UnbondNymNode` to succeed - this mirrors how the node-families test env deploys
/// the directory. Only compiled for this crate's own tests: `node-families` is a
/// dev-dependency, so this never becomes part of the `testable-directory-contract`
/// surface (which would form a normal-dependency cycle, since node-families depends
/// on this crate).
#[cfg(test)]
pub(crate) fn init_contract_tester_with_node_families() -> ContractTester<DirectoryContract> {
    use cosmwasm_std::coin;
    use node_families_contract::testing::NodeFamiliesContract;
    use nym_contracts_common_testing::TEST_DENOM;
    use nym_node_families_contract_common::{
        Config as NfConfig, InstantiateMsg as NfInstantiateMsg,
    };

    let mut builder = ContractTesterBuilder::new().instantiate::<MixnetContract>(None);
    let mixnet_address = builder
        .well_known_contracts
        .get(MixnetContract::NAME)
        .unwrap()
        .clone();

    builder.instantiate_contract::<NodeFamiliesContract>(Some(NfInstantiateMsg {
        config: NfConfig {
            create_family_fee: coin(100_000000, TEST_DENOM),
            family_name_length_limit: 20,
            family_description_length_limit: 200,
            default_invitation_validity_secs: 24 * 60 * 60,
        },
        mixnet_contract_address: mixnet_address.to_string(),
    }));

    let mut tester = builder
        .instantiate::<DirectoryContract>(Some(InstantiateMsg {
            mixnet_contract_address: mixnet_address.to_string(),
            initial_labels: vec![],
        }))
        .build()
        .with_common_storage_key(CommonStorageKeys::Admin, storage_keys::CONTRACT_ADMIN);

    // patch the mixnet's stored addresses to the real deployed contracts
    let directory_address = tester.contract_address.clone();
    let node_families_address = tester
        .well_known_contracts
        .get(NodeFamiliesContract::NAME)
        .unwrap()
        .clone();
    let mut mixnet_state: ContractState = tester
        .read_from_mixnet_contract_storage(MIXNET_CONTRACT_STATE_STORAGE_KEY)
        .expect("mixnet contract state should be loadable");
    mixnet_state.directory_contract_address = directory_address;
    mixnet_state.node_families_contract_address = node_families_address;
    tester
        .write_to_mixnet_contract_storage_value(MIXNET_CONTRACT_STATE_STORAGE_KEY, &mixnet_state)
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
    fn add_dummy_node_data(&mut self, node_id: NodeId, label: &str) {
        let height = self.env().block.height;
        NYM_DIRECTORY_CONTRACT_STORAGE
            .set_node_entry(
                self.storage_mut(),
                node_id,
                label,
                NodeEntry {
                    data: Binary::from(b"test".to_vec()),
                    updated_at_height: height,
                    sequence: 0,
                    signature: Binary::default(),
                },
            )
            .unwrap();
    }

    fn add_dummy_curated(&mut self, key: &str) {
        NYM_DIRECTORY_CONTRACT_STORAGE
            .set_curated_entry(
                self.storage_mut(),
                key,
                CuratedEntry {
                    data: Binary::from(b"test".to_vec()),
                },
            )
            .unwrap();
    }
}

impl DirectoryContractTesterExt for ContractTester<DirectoryContract> {}

// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::contract::{execute, instantiate, migrate, query};
use crate::helpers::normalise_family_name;
use crate::storage::NodeFamiliesStorage;
use cosmwasm_std::{coin, Addr, Coin, Storage};
use mixnet_contract::testable_mixnet_contract::{EmbeddedMixnetContractExt, MixnetContract};
use node_families_contract_common::constants::storage_keys;
use node_families_contract_common::{
    Config, ExecuteMsg, FamilyInvitation, InstantiateMsg, MigrateMsg, NodeFamiliesContractError,
    NodeFamily, NodeFamilyId, QueryMsg,
};
use nym_contracts_common_testing::{
    AdminExt, ArbitraryContractStorageReader, ArbitraryContractStorageWriter, BankExt, ChainOpts,
    CommonStorageKeys, ContractFn, ContractOpts, ContractTester, ContractTesterBuilder, DenomExt,
    PermissionedFn, QueryFn, RandExt, TestableNymContract, TEST_DENOM,
};
use nym_mixnet_contract_common::NodeId;

pub struct NodeFamiliesContract;

impl TestableNymContract for NodeFamiliesContract {
    const NAME: &'static str = "node-families-contract";
    type InitMsg = InstantiateMsg;
    type ExecuteMsg = ExecuteMsg;
    type QueryMsg = QueryMsg;
    type MigrateMsg = MigrateMsg;
    type ContractError = NodeFamiliesContractError;

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
                config: Config {
                    create_family_fee: coin(100_000000, TEST_DENOM),
                    family_name_length_limit: 20,
                    family_description_length_limit: 200,
                    default_invitation_validity_secs: 24 * 60 * 60,
                },
                mixnet_contract_address: mixnet_address.to_string(),
            }))
            .build()
    }
}

pub fn init_contract_tester() -> ContractTester<NodeFamiliesContract> {
    NodeFamiliesContract::init()
        .with_common_storage_key(CommonStorageKeys::Admin, storage_keys::CONTRACT_ADMIN)
}

pub trait NodeFamiliesContractTesterExt:
    ContractOpts<
        ExecuteMsg = ExecuteMsg,
        QueryMsg = QueryMsg,
        ContractError = NodeFamiliesContractError,
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
    fn family_fee(&self) -> Coin {
        let s = NodeFamiliesStorage::new();
        s.config.load(self).unwrap().create_family_fee
    }

    fn make_named_family(&mut self, owner: &Addr, name: &str) -> NodeFamily {
        let normalised = normalise_family_name(name);
        let env = self.env();
        let fee = self.family_fee();
        NodeFamiliesStorage::new()
            .register_new_family(
                self,
                &env,
                fee,
                owner.clone(),
                name.to_string(),
                normalised,
                "dummy".to_string(),
            )
            .unwrap()
    }

    fn make_family(&mut self, owner: &Addr) -> NodeFamily {
        // names must be globally unique; derive from owner addr (also unique)
        let name = format!("family-{owner}");
        self.make_named_family(owner, &name)
    }

    fn disband_family(&mut self, family: NodeFamilyId) {
        let env = self.env();
        NodeFamiliesStorage::new()
            .disband_family(self, &env, family)
            .unwrap();
    }

    fn add_dummy_family(&mut self) -> NodeFamily {
        let owner = self.generate_account();
        self.make_family(&owner)
    }

    fn invite_to_family_with_expiration(
        &mut self,
        family: NodeFamilyId,
        node: NodeId,
        expiration: u64,
    ) -> FamilyInvitation {
        NodeFamiliesStorage::new()
            .add_pending_invitation(self, family, node, expiration)
            .unwrap()
    }

    fn invite_to_family(&mut self, family: NodeFamilyId, node: NodeId) -> FamilyInvitation {
        let exp = self.env().block.time.seconds() + 100;
        self.invite_to_family_with_expiration(family, node, exp)
    }

    fn accept_invitation(&mut self, family: NodeFamilyId, node: NodeId) {
        let env = self.env();
        NodeFamiliesStorage::new()
            .accept_invitation(self, &env, family, node)
            .unwrap();
    }

    fn reject_invitation(&mut self, family: NodeFamilyId, node: NodeId) {
        let env = self.env();
        NodeFamiliesStorage::new()
            .reject_pending_invitation(self, &env, family, node)
            .unwrap();
    }

    fn revoke_invitation(&mut self, family: NodeFamilyId, node: NodeId) {
        let env = self.env();
        NodeFamiliesStorage::new()
            .revoke_pending_invitation(self, &env, family, node)
            .unwrap();
    }

    fn add_to_family(&mut self, family: NodeFamilyId, node: NodeId) {
        self.invite_to_family(family, node);
        self.accept_invitation(family, node);
    }

    fn remove_from_family(&mut self, node: NodeId) {
        let env = self.env();
        NodeFamiliesStorage::new()
            .remove_family_member(self, &env, node)
            .unwrap();
    }

    fn add_n_family_members(&mut self, family: NodeFamilyId, count: u32) {
        for n in 1..=count {
            self.add_to_family(family, n);
        }
    }
}

impl NodeFamiliesContractTesterExt for ContractTester<NodeFamiliesContract> {}

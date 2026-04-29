// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::contract::{execute, instantiate, migrate, query};
use crate::storage::NodeFamiliesStorage;
use cosmwasm_std::{Addr, MessageInfo, StdError, StdResult, Storage};
use node_families_contract_common::{
    ExecuteMsg, FamilyInvitation, InstantiateMsg, MigrateMsg, NodeFamiliesContractError,
    NodeFamily, NodeFamilyId, QueryMsg,
};
use nym_contracts_common_testing::{
    AdminExt, ArbitraryContractStorageReader, ArbitraryContractStorageWriter, ChainOpts,
    ContractFn, ContractOpts, ContractTester, DenomExt, PermissionedFn, QueryFn, RandExt,
    TestableNymContract,
};
use nym_mixnet_contract_common::NodeId;
use serde::{de::DeserializeOwned, Serialize};

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

    fn base_init_msg() -> Self::InitMsg {
        InstantiateMsg {}
    }
}

pub fn init_contract_tester() -> ContractTester<NodeFamiliesContract> {
    NodeFamiliesContract::init()
}

pub trait NodeFamiliesContractTesterExt:
    ContractOpts<
        ExecuteMsg = ExecuteMsg,
        QueryMsg = QueryMsg,
        ContractError = NodeFamiliesContractError,
    > + ChainOpts
    + AdminExt
    + DenomExt
    + RandExt
    + Storage
    + ArbitraryContractStorageReader
    + ArbitraryContractStorageWriter
    + Sized
{
    fn mixnet_contract_address(&self) -> StdResult<Addr> {
        NodeFamiliesStorage::new()
            .mixnet_contract_address
            .load(self.deps().storage)
    }

    fn execute_mixnet_contract(
        &mut self,
        sender: MessageInfo,
        msg: &nym_mixnet_contract_common::ExecuteMsg,
    ) -> StdResult<()> {
        let address = self.mixnet_contract_address()?;

        self.execute_arbitrary_contract(address, sender, msg)
            .map_err(|err| {
                StdError::generic_err(format!("mixnet contract execution failure: {err}"))
            })?;
        Ok(())
    }

    fn read_from_mixnet_contract_storage<T: DeserializeOwned>(
        &self,
        key: impl AsRef<[u8]>,
    ) -> StdResult<T> {
        let address = self.mixnet_contract_address()?;

        self.must_read_value_from_contract_storage(address, key)
    }

    fn write_to_mixnet_contract_storage(
        &mut self,
        key: impl AsRef<[u8]>,
        value: impl AsRef<[u8]>,
    ) -> StdResult<()> {
        let address = self.mixnet_contract_address()?;

        <Self as ArbitraryContractStorageWriter>::set_contract_storage(self, address, key, value);
        Ok(())
    }

    fn write_to_mixnet_contract_storage_value<T: Serialize>(
        &mut self,
        key: impl AsRef<[u8]>,
        value: &T,
    ) -> StdResult<()> {
        let address = self.mixnet_contract_address()?;

        self.set_contract_storage_value(address, key, value)
    }

    fn make_family(&mut self, owner: &Addr) -> NodeFamily {
        let env = self.env();
        NodeFamiliesStorage::new()
            .register_new_family(
                self,
                &env,
                owner.clone(),
                "dummy".to_string(),
                "dummy".to_string(),
            )
            .unwrap()
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

    fn add_to_family(&mut self, family: NodeFamilyId, node: NodeId) {
        self.invite_to_family(family, node);
        self.accept_invitation(family, node);
    }
}

impl NodeFamiliesContractTesterExt for ContractTester<NodeFamiliesContract> {}

// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// that's fine in test code
#![allow(clippy::panic)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use crate::contract::{execute, instantiate, migrate, query};
use crate::helpers::{group_members, DkgContractQuerier};
use crate::storage::NymOfflineSignersStorage;
use cosmwasm_std::Addr;
use cw4::Cw4Contract;
use nym_coconut_dkg::testable_dkg_contract::DkgContract;
use nym_contracts_common_testing::{
    AdminExt, ArbitraryContractStorageReader, ArbitraryContractStorageWriter, BankExt, ChainOpts,
    CommonStorageKeys, ContractFn, ContractOpts, ContractTester, DenomExt, PermissionedFn, QueryFn,
    RandExt, SliceRandom, TestableNymContract,
};
use nym_offline_signers_contract_common::constants::storage_keys;
use nym_offline_signers_contract_common::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, NymOfflineSignersContractError, Proposal, ProposalId,
    QueryMsg,
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
    init_custom_contract_tester(
        members,
        InstantiateMsg {
            dkg_contract_address: "PLACEHOLDER".to_string(),
            config: Default::default(),
        },
    )
}

// this will OVERWRITE placeholder you put for dkg contract address with correct value
pub(crate) fn init_custom_contract_tester(
    members: usize,
    mut instantiate_msg: InstantiateMsg,
) -> ContractTester<OfflineSignersContract> {
    // prepare the dkg contract and using that initial setup, add the offline signers contract
    let builder =
        nym_coconut_dkg::testable_dkg_contract::prepare_contract_tester_builder_with_group_members(
            members,
        );

    // we just instantiated it
    let dkg_contract_address = builder.unchecked_contract_address::<DkgContract>();
    instantiate_msg.dkg_contract_address = dkg_contract_address.to_string();

    // 5. finally init the offline signers contract
    builder
        .instantiate::<OfflineSignersContract>(Some(instantiate_msg))
        .build()
}

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
    fn group_contract_wrapper(&self) -> Cw4Contract {
        let storage = NymOfflineSignersStorage::new();
        let dkg_contract_address = storage.dkg_contract.load(self.storage()).unwrap();
        Cw4Contract::new(
            self.deps()
                .querier
                .query_dkg_cw4_contract_address(dkg_contract_address)
                .unwrap(),
        )
    }

    fn group_members(&self) -> Vec<Addr> {
        let querier = self.deps().querier;
        let group_contract = self.group_contract_wrapper();
        group_members(&querier, &group_contract).unwrap()
    }

    fn random_group_member(&mut self) -> Addr {
        let members = self.group_members();
        members
            .choose(&mut self.raw_rng())
            .expect("no group members available")
            .clone()
    }

    #[track_caller]
    fn add_votes(&mut self, proposal_id: ProposalId) {
        let storage = NymOfflineSignersStorage::new();
        let members = self.group_members();
        let proposal = storage.proposals.load(self.storage(), proposal_id).unwrap();
        for member in members {
            // check if we already voted
            if !storage.votes.has(self.storage(), (proposal_id, &member)) {
                let env = self.env();
                storage
                    .propose_or_vote(
                        self.deps_mut(),
                        env,
                        member,
                        proposal.proposed_offline_signer.clone(),
                    )
                    .unwrap();
            }
        }
    }

    #[track_caller]
    fn add_vote(&mut self, proposal_id: ProposalId, voter: &Addr) {
        let storage = NymOfflineSignersStorage::new();
        let proposal = storage.proposals.load(self.storage(), proposal_id).unwrap();

        let env = self.env();
        storage
            .propose_or_vote(
                self.deps_mut(),
                env,
                voter.clone(),
                proposal.proposed_offline_signer.clone(),
            )
            .unwrap();
    }

    fn next_proposal_id(&self) -> ProposalId {
        NymOfflineSignersStorage::new()
            .proposal_count
            .may_load(self.storage())
            .unwrap()
            .unwrap_or_default()
            + 1
    }

    #[track_caller]
    fn make_proposal(&mut self, target: &Addr) -> ProposalId {
        let proposer = self.random_group_member();
        let storage = NymOfflineSignersStorage::new();
        let id = self.next_proposal_id();

        let env = self.env();
        storage
            .propose_or_vote(self.deps_mut(), env, proposer, target.clone())
            .unwrap();

        id
    }

    fn load_proposal(&mut self, proposal_id: ProposalId) -> Option<Proposal> {
        NymOfflineSignersStorage::new()
            .proposals
            .may_load(self.storage(), proposal_id)
            .unwrap()
    }

    #[track_caller]
    fn insert_empty_proposal(&mut self, target: &Addr) -> ProposalId {
        let proposer = self.generate_account();
        let storage = NymOfflineSignersStorage::new();

        let env = self.env();
        storage
            .insert_new_active_proposal(self.storage_mut(), &env, &proposer, target)
            .unwrap()
    }

    #[track_caller]
    fn insert_offline_signer(&mut self, signer: &Addr) -> ProposalId {
        let proposal_id = self.make_proposal(signer);
        self.add_votes(proposal_id);
        proposal_id
    }

    #[track_caller]
    fn reset_offline_status(&mut self, signer: &Addr) {
        let storage = NymOfflineSignersStorage::new();
        let env = self.env();
        storage
            .reset_offline_status(self.deps_mut(), env, signer.clone())
            .unwrap();
    }
}

impl OfflineSignersContractTesterExt for ContractTester<OfflineSignersContract> {}

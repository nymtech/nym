// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// fine in test code
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use crate::contract::{execute, instantiate, migrate, query};
use crate::error::ContractError;
use cosmwasm_std::testing::message_info;
use cosmwasm_std::Addr;
use cw4::{Cw4Contract, Member};
use nym_contracts_common_testing::{
    AdminExt, ArbitraryContractStorageReader, ArbitraryContractStorageWriter, BankExt, ChainOpts,
    CommonStorageKeys, ContractFn, ContractOpts, ContractTester, ContractTesterBuilder, DenomExt,
    PermissionedFn, QueryFn, RandExt, SliceRandom, TEST_DENOM,
};

use crate::dealings::storage::{StoredDealing, DEALINGS_METADATA};
use crate::epoch_state::storage::load_current_epoch;
use crate::state::storage::{MULTISIG, STATE};
use crate::testable_dkg_contract::helpers::group_members;
use crate::verification_key_shares::storage::vk_shares;
use nym_coconut_dkg_common::dealing::{DealingChunkInfo, PartialContractDealing};
use nym_coconut_dkg_common::types::{ChunkIndex, DealingIndex, Epoch, EpochId, EpochState};
use nym_coconut_dkg_common::verification_key::{ContractVKShare, VerificationKeyShare};
use nym_contracts_common::dealings::ContractSafeBytes;

pub use cw3_flex_multisig::testable_cw3_contract::{Duration, MultisigContract, Threshold};
pub use cw4_group::testable_cw4_contract::GroupContract;
pub use nym_coconut_dkg_common::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
pub use nym_contracts_common_testing::TestableNymContract;

pub(crate) mod helpers;

pub struct DkgContract;

const DEFAULT_GROUP_MEMBERS: usize = 15;

impl TestableNymContract for DkgContract {
    const NAME: &'static str = "dkg-contract";
    type InitMsg = InstantiateMsg;
    type ExecuteMsg = ExecuteMsg;
    type QueryMsg = QueryMsg;
    type MigrateMsg = MigrateMsg;
    type ContractError = ContractError;

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
        init_contract_tester()
    }
}

pub fn init_contract_tester() -> ContractTester<DkgContract> {
    init_contract_tester_with_group_members(DEFAULT_GROUP_MEMBERS)
}

pub fn init_contract_tester_with_group_members(members: usize) -> ContractTester<DkgContract> {
    prepare_contract_tester_builder_with_group_members(members)
        .build()
        .with_common_storage_key(CommonStorageKeys::Admin, "dkg-admin")
}

pub fn prepare_contract_tester_builder_with_group_members<C>(
    members: usize,
) -> ContractTesterBuilder<C>
where
    C: TestableNymContract,
{
    let mut builder = ContractTesterBuilder::<C>::new();
    let api = builder.api();

    // 1. init the CW4 group contract
    let group_init_msg = cw4_group::testable_cw4_contract::InstantiateMsg {
        admin: Some(builder.master_address().to_string()),
        members: (0..members)
            .map(|i| Member {
                addr: api.addr_make(&format!("group-member-{i}")).to_string(),
                weight: 1,
            })
            .collect(),
    };
    builder.instantiate_contract::<GroupContract>(Some(group_init_msg));

    // we just instantiated it
    let group_contract_address = builder.unchecked_contract_address::<GroupContract>();

    // 2. init the CW3 multisig contract WITH DUMMY VALUES
    let multisig_init_msg = cw3_flex_multisig::testable_cw3_contract::InstantiateMsg {
        group_addr: group_contract_address.to_string(),
        // \/ PLACEHOLDERS
        coconut_bandwidth_contract_address: group_contract_address.to_string(),
        coconut_dkg_contract_address: group_contract_address.to_string(),
        // /\ PLACEHOLDERS
        threshold: Threshold::AbsolutePercentage {
            percentage: "0.67".parse().unwrap(),
        },
        max_voting_period: Duration::Time(3600),
        executor: None,
        proposal_deposit: None,
    };
    builder.instantiate_contract::<MultisigContract>(Some(multisig_init_msg));

    // we just instantiated it
    let multisig_contract_address = builder.unchecked_contract_address::<MultisigContract>();

    // 3. init the DKG contract
    let dkg_init_msg = InstantiateMsg {
        group_addr: group_contract_address.to_string(),
        multisig_addr: multisig_contract_address.to_string(),
        time_configuration: None,
        mix_denom: TEST_DENOM.to_string(),
        key_size: 5,
    };
    builder.instantiate_contract::<DkgContract>(Some(dkg_init_msg));

    // we just instantiated it
    let dkg_contract_address = builder.unchecked_contract_address::<DkgContract>();

    // 4. migrate the multisig contract to hold correct addresses
    let multisig_migrate_msg = cw3_flex_multisig::testable_cw3_contract::MigrateMsg {
        // \/ STILL A PLACEHOLDER (this contract does not care about interactions with the ecash contract)
        coconut_bandwidth_address: dkg_contract_address.to_string(),
        // /\ STILL A PLACEHOLDER
        coconut_dkg_address: dkg_contract_address.to_string(),
    };
    builder.migrate_contract::<MultisigContract>(&multisig_migrate_msg);
    builder
}

pub trait DkgContractTesterExt:
    ContractOpts<ExecuteMsg = ExecuteMsg, QueryMsg = QueryMsg, ContractError = ContractError>
    + ChainOpts
    + AdminExt
    + DenomExt
    + RandExt
    + BankExt
    + ArbitraryContractStorageReader
    + ArbitraryContractStorageWriter
{
    fn epoch(&self) -> Epoch {
        load_current_epoch(self.storage()).unwrap()
    }

    fn multisig_contract(&self) -> Addr {
        MULTISIG.get(self.deps()).unwrap().unwrap()
    }

    fn group_contract_wrapper(&self) -> Cw4Contract {
        STATE.load(self.storage()).unwrap().group_addr
    }

    fn remove_group_member(&mut self, addr: Addr) {
        // we have the same admin for all contracts
        let admin = self.admin().unwrap();

        self.execute_arbitrary_contract(
            self.unchecked_contract_address::<GroupContract>(),
            message_info(&admin, &[]),
            &nym_group_contract_common::msg::ExecuteMsg::UpdateMembers {
                remove: vec![addr.to_string()],
                add: vec![],
            },
        )
        .unwrap();
    }

    fn add_group_member(&mut self, addr: Addr) {
        let querier = self.deps().querier;

        let members = self
            .group_contract_wrapper()
            .list_members(&querier, None, None)
            .unwrap();
        let weight = members.first().map(|m| m.weight).unwrap_or(1);

        // we have the same admin for all contracts
        let admin = self.admin().unwrap();

        self.execute_arbitrary_contract(
            self.unchecked_contract_address::<GroupContract>(),
            message_info(&admin, &[]),
            &nym_group_contract_common::msg::ExecuteMsg::UpdateMembers {
                remove: vec![],
                add: vec![Member {
                    addr: addr.to_string(),
                    weight,
                }],
            },
        )
        .unwrap();
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

    fn key_size(&self) -> u32 {
        STATE.load(self.storage()).unwrap().key_size
    }

    /// The chunk indices a dealer committed for a given dealing, in ascending order.
    fn submitted_chunk_indices(
        &self,
        epoch_id: EpochId,
        dealer: &Addr,
        dealing_index: DealingIndex,
    ) -> Vec<ChunkIndex> {
        DEALINGS_METADATA
            .may_load(self.storage(), (epoch_id, dealer, dealing_index))
            .unwrap()
            .map(|metadata| metadata.submitted_chunks.into_keys().collect())
            .unwrap_or_default()
    }

    /// Rewrite the raw bytes of a stored dealing chunk.
    ///
    /// Dealing chunks are written to storage as raw bytes bypassing serialisation, so
    /// this reaches past the contract's own handlers deliberately: it fabricates
    /// on-chain data that a well-behaved dealer would never submit, which is exactly
    /// what a test of *recipient-side* dealing validation needs.
    fn mutate_dealing_chunk<F>(
        &mut self,
        epoch_id: EpochId,
        dealer: &Addr,
        dealing_index: DealingIndex,
        chunk_index: ChunkIndex,
        mutate: F,
    ) where
        F: FnOnce(&mut Vec<u8>),
    {
        let mut data =
            StoredDealing::read(self.storage(), epoch_id, dealer, dealing_index, chunk_index)
                .expect("attempted to corrupt a dealing chunk that was never submitted")
                .0;
        mutate(&mut data);
        StoredDealing::save(
            self.storage_mut(),
            epoch_id,
            dealer,
            PartialContractDealing {
                dealing_index,
                chunk_index,
                data: ContractSafeBytes(data),
            },
        );
    }

    /// Drop the final byte of one chunk, so the reassembled dealing no longer decodes.
    fn truncate_dealing_chunk(
        &mut self,
        epoch_id: EpochId,
        dealer: &Addr,
        dealing_index: DealingIndex,
        chunk_index: ChunkIndex,
    ) {
        self.mutate_dealing_chunk(epoch_id, dealer, dealing_index, chunk_index, |data| {
            data.pop()
                .expect("attempted to truncate an already empty dealing chunk");
        })
    }

    /// Drop the final byte of every chunk of every dealing submitted by `dealer`.
    fn truncate_all_dealings(&mut self, epoch_id: EpochId, dealer: &Addr) {
        for dealing_index in 0..self.key_size() {
            for chunk_index in self.submitted_chunk_indices(epoch_id, dealer, dealing_index) {
                self.truncate_dealing_chunk(epoch_id, dealer, dealing_index, chunk_index);
            }
        }
    }

    /// Alter the final byte of a dealing's last chunk, keeping its length intact.
    ///
    /// Unlike truncation, the dealing still decodes - it simply fails cryptographic
    /// verification, which is a different rejection path on the recipient side.
    fn corrupt_dealing_payload(
        &mut self,
        epoch_id: EpochId,
        dealer: &Addr,
        dealing_index: DealingIndex,
    ) {
        let last_chunk = *self
            .submitted_chunk_indices(epoch_id, dealer, dealing_index)
            .last()
            .expect("the dealer submitted no chunks for this dealing");

        self.mutate_dealing_chunk(epoch_id, dealer, dealing_index, last_chunk, |data| {
            let last = data
                .last_mut()
                .expect("attempted to corrupt an empty dealing chunk");
            *last = if *last == 42 { 43 } else { 42 };
        })
    }

    fn vk_share(&self, epoch_id: EpochId, owner: &Addr) -> Option<ContractVKShare> {
        vk_shares()
            .may_load(self.storage(), (owner, epoch_id))
            .unwrap()
    }

    /// Overwrite a dealer's submitted verification key share.
    fn replace_vk_share(&mut self, epoch_id: EpochId, owner: &Addr, share: ContractVKShare) {
        vk_shares()
            .save(self.storage_mut(), (owner, epoch_id), &share)
            .unwrap()
    }

    /// Corrupt a dealer's stored verification key share with the supplied mutation, so
    /// that it no longer pairs with the dealings that dealer actually distributed.
    fn corrupt_vk_share<F>(&mut self, epoch_id: EpochId, owner: &Addr, mutate: F)
    where
        F: FnOnce(&mut VerificationKeyShare),
    {
        let mut share = self
            .vk_share(epoch_id, owner)
            .expect("the dealer submitted no verification key share");
        mutate(&mut share.share);
        self.replace_vk_share(epoch_id, owner, share);
    }

    fn dummy_dkg_steps(&mut self, resharing: bool) {
        let admin = self.admin().unwrap();
        let group_members = self.group_members();

        // 2. register dealers
        for group_member in &group_members {
            self.execute_msg(
                group_member.clone(),
                &ExecuteMsg::RegisterDealer {
                    bte_key_with_proof: format!("btekey-{group_member}"),
                    identity_key: format!("identity-{group_member}"),
                    announce_address: format!("announce-address-{group_member}"),
                    resharing,
                },
            )
            .unwrap();
        }

        // PublicKeySubmission => DealingExchange
        self.advance_time_by(600);
        self.execute_msg(admin.clone(), &ExecuteMsg::AdvanceEpochState {})
            .unwrap();
        assert_eq!(
            self.epoch().state,
            EpochState::DealingExchange { resharing }
        );

        // 3. exchange dealings
        for group_member in &group_members {
            self.execute_msg(
                group_member.clone(),
                &ExecuteMsg::CommitDealingsMetadata {
                    dealing_index: 1,
                    chunks: vec![DealingChunkInfo { size: 1 }],
                    resharing,
                },
            )
            .unwrap();
            self.execute_msg(
                group_member.clone(),
                &ExecuteMsg::CommitDealingsChunk {
                    chunk: PartialContractDealing {
                        dealing_index: 1,
                        chunk_index: 0,
                        data: ContractSafeBytes(vec![0]),
                    },
                },
            )
            .unwrap();
        }

        // DealingExchange => VerificationKeySubmission
        self.advance_time_by(300);
        self.execute_msg(admin.clone(), &ExecuteMsg::AdvanceEpochState {})
            .unwrap();
        assert_eq!(
            self.epoch().state,
            EpochState::VerificationKeySubmission { resharing }
        );

        // 4. derive keypairs
        for group_member in &group_members {
            self.execute_msg(
                group_member.clone(),
                &ExecuteMsg::CommitVerificationKeyShare {
                    share: format!("partial-vk-{group_member}"),
                    resharing,
                },
            )
            .unwrap();
        }

        // VerificationKeySubmission => VerificationKeyValidation
        self.execute_msg(admin.clone(), &ExecuteMsg::AdvanceEpochState {})
            .unwrap();
        self.advance_time_by(60);
        assert_eq!(
            self.epoch().state,
            EpochState::VerificationKeyValidation { resharing }
        );

        // VerificationKeyValidation => VerificationKeyFinalization
        self.execute_msg(admin.clone(), &ExecuteMsg::AdvanceEpochState {})
            .unwrap();
        assert_eq!(
            self.epoch().state,
            EpochState::VerificationKeyFinalization { resharing }
        );

        // 5. validate keys
        for group_member in &group_members {
            self.execute_msg(
                self.multisig_contract(),
                &ExecuteMsg::VerifyVerificationKeyShare {
                    owner: group_member.to_string(),
                    resharing,
                    epoch_id: self.epoch().epoch_id,
                },
            )
            .unwrap();
        }

        // VerificationKeyFinalization => InProgress
        self.execute_msg(admin.clone(), &ExecuteMsg::AdvanceEpochState {})
            .unwrap();
        assert_eq!(self.epoch().state, EpochState::InProgress)
    }

    fn run_initial_dummy_dkg(&mut self) {
        assert_eq!(self.epoch().state, EpochState::WaitingInitialisation);
        // 1. initiate DKG
        // WaitingInitialisation => PublicKeySubmission
        let admin = self.admin().unwrap();
        self.execute_msg(admin.clone(), &ExecuteMsg::InitiateDkg {})
            .unwrap();
        assert_eq!(
            self.epoch().state,
            EpochState::PublicKeySubmission { resharing: false }
        );

        self.dummy_dkg_steps(false)
    }

    fn run_reset_dkg(&mut self) {
        // 1. reset DKG
        // InProgress => PublicKeySubmission
        let admin = self.admin().unwrap();
        self.execute_msg(admin.clone(), &ExecuteMsg::TriggerReset {})
            .unwrap();
        assert_eq!(
            self.epoch().state,
            EpochState::PublicKeySubmission { resharing: false }
        );
        self.dummy_dkg_steps(false)
    }

    fn run_resharing_dkg(&mut self) {
        assert_eq!(self.epoch().state, EpochState::InProgress);

        let group_members = self.group_members();
        println!(
            "epoch: {} members: {}",
            self.epoch().epoch_id,
            group_members.len()
        );

        // 1. initiate DKG
        // InProgress => PublicKeySubmission
        let admin = self.admin().unwrap();
        self.execute_msg(admin.clone(), &ExecuteMsg::TriggerResharing {})
            .unwrap();
        assert_eq!(
            self.epoch().state,
            EpochState::PublicKeySubmission { resharing: true }
        );
        self.dummy_dkg_steps(true)
    }
}

impl DkgContractTesterExt for ContractTester<DkgContract> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dealers::storage::EPOCH_DEALERS_MAP;

    #[test]
    fn dummy_resharing() {
        let mut contract = init_contract_tester_with_group_members(10);
        contract.run_initial_dummy_dkg();

        let dealer = contract.random_group_member();
        let details = EPOCH_DEALERS_MAP
            .may_load(contract.storage(), (0, &dealer))
            .unwrap();
        assert!(details.is_some());

        assert_eq!(contract.epoch().epoch_id, 0);

        contract.run_resharing_dkg();
        assert_eq!(contract.epoch().epoch_id, 1);
    }
}

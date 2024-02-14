// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::helpers::{contract_bandwidth, contract_dkg, contract_group, contract_multisig};
use cosmwasm_std::testing::mock_dependencies;
use cosmwasm_std::{coins, Addr, Api, CanonicalAddr};
use cw3::{Cw3Contract, ProposalListResponse, Status, Vote};
use cw4::{Cw4Contract, Member};
use cw_multi_test::{App, AppBuilder, Executor};
use cw_utils::{Duration, Threshold};
use nym_coconut_bandwidth_contract_common::msg::InstantiateMsg as BandwidthInstantiateMsg;
use nym_coconut_dkg_common::dealing::{chunk_dealing, DealingChunkInfo, MAX_DEALING_CHUNK_SIZE};
use nym_coconut_dkg_common::msg::ExecuteMsg as DkgExecuteMsg;
use nym_coconut_dkg_common::msg::InstantiateMsg as DkgInstantiateMsg;
use nym_coconut_dkg_common::msg::QueryMsg as DkgQueryMsg;
use nym_coconut_dkg_common::types::{Epoch, State};
use nym_group_contract_common::msg::InstantiateMsg as GroupInstantiateMsg;
use nym_multisig_contract_common::msg::InstantiateMsg as MultisigInstantiateMsg;
use nym_multisig_contract_common::msg::MigrateMsg as MultisigMigrateMsg;
use rand_chacha::rand_core::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use subtle_encoding::bech32;

pub const PREFIX: &str = "n";
pub const TEST_DENOM: &str = "unym";

pub const BANDWIDTH_POOL: &str = "pool";

pub const BLOCK_TIME_SECS: u64 = 6;

pub fn test_rng() -> ChaCha20Rng {
    let dummy_seed = [42u8; 32];
    ChaCha20Rng::from_seed(dummy_seed)
}

pub fn random_address(rng: &mut ChaCha20Rng) -> Addr {
    let mut bytes = [0u8; 32];
    rng.fill_bytes(&mut bytes);

    Addr::unchecked(bech32::encode(PREFIX, bytes))
}

pub struct TestSetup {
    pub app: App,
    pub rng: ChaCha20Rng,
    pub global_admin: Addr,

    pub multisig_contract: Cw3Contract,
    pub group_contract: Cw4Contract,
    pub dkg_contract: Addr,
    pub bandwidth_contract: Addr,
}

impl TestSetup {
    pub fn new() -> Self {
        let mut rng = test_rng();

        let global_admin = random_address(&mut rng);
        let mut app = AppBuilder::new().build(|router, _, storage| {
            router
                .bank
                .init_balance(storage, &global_admin, coins(1000000000000000, TEST_DENOM))
                .unwrap();
        });

        let group_id = app.store_code(contract_group());
        let multisig_id = app.store_code(contract_multisig());
        let bandwidth_id = app.store_code(contract_bandwidth());
        let dkg_id = app.store_code(contract_dkg());

        // 1. init group contract
        let group_contract = app
            .instantiate_contract(
                group_id,
                global_admin.clone(),
                &GroupInstantiateMsg {
                    admin: Some(global_admin.to_string()),
                    members: vec![],
                },
                &[],
                "group contract",
                Some(global_admin.to_string()),
            )
            .unwrap();

        // 2. init multisig contract
        let multisig_contract = app
            .instantiate_contract(
                multisig_id,
                global_admin.clone(),
                &MultisigInstantiateMsg {
                    group_addr: group_contract.to_string(),
                    coconut_bandwidth_contract_address: group_contract.to_string(),
                    coconut_dkg_contract_address: group_contract.to_string(),
                    threshold: Threshold::AbsolutePercentage {
                        percentage: "0.67".parse().unwrap(),
                    },
                    max_voting_period: Duration::Time(3600),
                    executor: None,
                    proposal_deposit: None,
                },
                &[],
                "multisig contract",
                Some(global_admin.to_string()),
            )
            .unwrap();

        // 3. init bandwidth contract
        let bandwidth_contract = app
            .instantiate_contract(
                bandwidth_id,
                global_admin.clone(),
                &BandwidthInstantiateMsg {
                    multisig_addr: multisig_contract.to_string(),
                    pool_addr: BANDWIDTH_POOL.to_string(),
                    mix_denom: TEST_DENOM.to_string(),
                },
                &[],
                "bandwidth contract",
                Some(global_admin.to_string()),
            )
            .unwrap();

        // 4. init dkg contract
        let dkg_contract = app
            .instantiate_contract(
                dkg_id,
                global_admin.clone(),
                &DkgInstantiateMsg {
                    group_addr: group_contract.to_string(),
                    multisig_addr: multisig_contract.to_string(),
                    time_configuration: None,
                    mix_denom: TEST_DENOM.to_string(),
                    key_size: 5,
                },
                &[],
                "dkg contract",
                Some(global_admin.to_string()),
            )
            .unwrap();

        // 5.migrate multisig contract with addresses of bandwidth and dkg contracts
        app.migrate_contract(
            global_admin.clone(),
            multisig_contract.clone(),
            &MultisigMigrateMsg {
                coconut_bandwidth_address: bandwidth_contract.to_string(),
                coconut_dkg_address: dkg_contract.to_string(),
            },
            multisig_id,
        )
        .unwrap();

        TestSetup {
            app,
            rng,
            global_admin,
            multisig_contract: Cw3Contract(multisig_contract),
            group_contract: Cw4Contract(group_contract),
            dkg_contract,
            bandwidth_contract,
        }
    }

    pub fn random_address(&mut self) -> Addr {
        random_address(&mut self.rng)
    }

    pub fn next_block(&mut self) {
        self.app.update_block(|block| {
            block.height += 1;
            block.time = block.time.plus_seconds(BLOCK_TIME_SECS);
        })
    }

    // if we ever want to expand those tests, those queries should be moved to contract specific structs
    // (kinda like what cw4 has)
    pub fn dkg_state(&self) -> State {
        self.app
            .wrap()
            .query_wasm_smart(&self.dkg_contract, &DkgQueryMsg::GetState {})
            .unwrap()
    }

    pub fn epoch(&self) -> Epoch {
        self.app
            .wrap()
            .query_wasm_smart(&self.dkg_contract, &DkgQueryMsg::GetCurrentEpochState {})
            .unwrap()
    }

    // TODO: this will not go beyond first page
    pub fn all_proposals(&self) -> ProposalListResponse {
        self.app
            .wrap()
            .query_wasm_smart(
                &self.multisig_contract.0,
                &nym_multisig_contract_common::msg::QueryMsg::ListProposals {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap()
    }

    pub fn admin(&self) -> Addr {
        self.global_admin.clone()
    }

    pub fn add_mock_group_member(&mut self, weight: Option<u64>) -> Addr {
        let member_addr = self.random_address();
        let weight = weight.unwrap_or(10);

        self.app
            .execute_contract(
                self.admin(),
                self.group_contract.addr(),
                &nym_group_contract_common::msg::ExecuteMsg::UpdateMembers {
                    remove: vec![],
                    add: vec![Member {
                        addr: member_addr.to_string(),
                        weight,
                    }],
                },
                &[],
            )
            .unwrap();

        member_addr
    }

    pub fn begin_dkg(&mut self) {
        self.app
            .execute_contract(
                self.admin(),
                self.dkg_contract.clone(),
                &DkgExecuteMsg::InitiateDkg {},
                &[],
            )
            .unwrap();
    }

    pub fn skip_to_dkg_state_end(&mut self) {
        let epoch = self.epoch();

        self.app.update_block(|block| {
            block.height += 42;
            if let Some(finish_timestamp) = epoch.finish_timestamp {
                block.time = finish_timestamp.plus_seconds(BLOCK_TIME_SECS);
            } else {
                block.time = block.time.plus_seconds(BLOCK_TIME_SECS)
            }
        });
    }

    pub fn advance_dkg_epoch(&mut self) {
        self.skip_to_dkg_state_end();
        self.unchecked_advance_dkg_epoch();
    }

    pub fn unchecked_advance_dkg_epoch(&mut self) {
        self.app
            .execute_contract(
                self.admin(),
                self.dkg_contract.clone(),
                &DkgExecuteMsg::AdvanceEpochState {},
                &[],
            )
            .unwrap();
    }

    pub fn submit_dummy_dkg_keys(&mut self, member: &Addr, resharing: bool) {
        let mut bte_key_with_proof = [0u8; 32];
        self.rng.fill_bytes(&mut bte_key_with_proof);
        let bte_key_with_proof = bs58::encode(&bte_key_with_proof).into_string();

        let mut identity_key = [0u8; 32];
        self.rng.fill_bytes(&mut identity_key);
        let identity_key = bs58::encode(&identity_key).into_string();

        let mut announce_address = [0u8; 16];
        self.rng.fill_bytes(&mut announce_address);
        let announce_address = bs58::encode(&announce_address).into_string();

        self.app
            .execute_contract(
                member.clone(),
                self.dkg_contract.clone(),
                &DkgExecuteMsg::RegisterDealer {
                    bte_key_with_proof,
                    identity_key,
                    announce_address,
                    resharing,
                },
                &[],
            )
            .unwrap();
    }

    pub fn submit_dummy_dealings(&mut self, member: &Addr, resharing: bool) {
        let dealings = self.dkg_state().key_size;

        for dealing_index in 0..dealings {
            let mut dealing_bytes = vec![0u8; 5000];
            self.rng.fill_bytes(&mut dealing_bytes);

            let chunks = DealingChunkInfo::construct(dealing_bytes.len(), MAX_DEALING_CHUNK_SIZE);
            self.app
                .execute_contract(
                    member.clone(),
                    self.dkg_contract.clone(),
                    &DkgExecuteMsg::CommitDealingsMetadata {
                        dealing_index,
                        chunks,
                        resharing,
                    },
                    &[],
                )
                .unwrap();
            self.next_block();

            let chunks = chunk_dealing(dealing_index, dealing_bytes, MAX_DEALING_CHUNK_SIZE);
            for (_, chunk) in chunks {
                self.app
                    .execute_contract(
                        member.clone(),
                        self.dkg_contract.clone(),
                        &DkgExecuteMsg::CommitDealingsChunk { chunk, resharing },
                        &[],
                    )
                    .unwrap();
                self.next_block();
            }
        }
    }

    pub fn submit_dummy_vk_key(&mut self, member: &Addr, resharing: bool) {
        let mut derived_vk = vec![0u8; 256];
        self.rng.fill_bytes(&mut derived_vk);

        let share = bs58::encode(&derived_vk).into_string();

        self.app
            .execute_contract(
                member.clone(),
                self.dkg_contract.clone(),
                &DkgExecuteMsg::CommitVerificationKeyShare { share, resharing },
                &[],
            )
            .unwrap();
    }

    pub fn validate_dummy_keys(&mut self, member: &Addr) {
        for proposal in self.all_proposals().proposals {
            if proposal.status == Status::Open {
                self.app
                    .execute_contract(
                        member.clone(),
                        self.multisig_contract.addr(),
                        &nym_multisig_contract_common::msg::ExecuteMsg::Vote {
                            proposal_id: proposal.id,
                            vote: Vote::Yes,
                        },
                        &[],
                    )
                    .unwrap();
            }
        }
    }

    pub fn finalize_dummy_dkg(&mut self) {
        for proposal in self.all_proposals().proposals {
            assert_eq!(proposal.status, Status::Passed);

            self.app
                .execute_contract(
                    self.admin(),
                    self.multisig_contract.addr(),
                    &nym_multisig_contract_common::msg::ExecuteMsg::Execute {
                        proposal_id: proposal.id,
                    },
                    &[],
                )
                .unwrap();
        }
    }

    pub fn full_dummy_dkg(&mut self, members: Vec<Addr>, resharing: bool) {
        for member in &members {
            self.submit_dummy_dkg_keys(member, resharing);
            self.next_block();
        }
        self.advance_dkg_epoch();

        for member in &members {
            self.submit_dummy_dealings(member, resharing);
            self.next_block();
        }
        self.advance_dkg_epoch();

        for member in &members {
            self.submit_dummy_vk_key(member, resharing);
            self.next_block();
        }
        self.advance_dkg_epoch();

        for member in &members {
            self.validate_dummy_keys(member);
            self.next_block();
        }
        self.advance_dkg_epoch();

        self.finalize_dummy_dkg();
        self.next_block();

        self.advance_dkg_epoch();
    }
}

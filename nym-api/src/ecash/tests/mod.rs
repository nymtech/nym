// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::error::{CoconutError, Result};
use crate::ecash::keys::KeyPairWithEpoch;
use crate::ecash::state::State;
use crate::ecash::storage::CoconutStorageExt;
use crate::support::storage::NymApiStorage;
use async_trait::async_trait;
use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, BlockInfo, CosmosMsg, Decimal, MessageInfo, WasmMsg,
};
use cw3::{Proposal, ProposalResponse, Vote, VoteInfo, VoteResponse, Votes};
use cw4::{Cw4Contract, MemberResponse};
use nym_api_requests::coconut::models::{IssuedCredentialBody, IssuedCredentialResponse};
use nym_api_requests::coconut::{BlindSignRequestBody, BlindedSignatureResponse};
use nym_coconut_dkg_common::dealer::{
    DealerDetails, DealerDetailsResponse, DealerType, RegisteredDealerDetails,
};
use nym_coconut_dkg_common::dealing::{
    DealerDealingsStatusResponse, DealingChunkInfo, DealingMetadata, DealingStatus,
    DealingStatusResponse, PartialContractDealing,
};
use nym_coconut_dkg_common::event_attributes::{DKG_PROPOSAL_ID, NODE_INDEX};
use nym_coconut_dkg_common::types::{
    ChunkIndex, DealerRegistrationDetails, DealingIndex, EncodedBTEPublicKeyWithProof, Epoch,
    EpochId, EpochState, PartialContractDealingData, State as ContractState,
};
use nym_coconut_dkg_common::verification_key::{ContractVKShare, VerificationKeyShare};
use nym_compact_ecash::BlindedSignature;
use nym_compact_ecash::{ttp_keygen, VerificationKeyAuth};
use nym_contracts_common::IdentityKey;
use nym_credentials::coconut::bandwidth::voucher::BandwidthVoucherIssuanceData;
use nym_credentials::IssuanceBandwidthCredential;
use nym_credentials_interface::CredentialType;
use nym_crypto::asymmetric::identity;
use nym_dkg::{NodeIndex, Threshold};
use nym_ecash_contract_common::blacklist::{BlacklistedAccountResponse, Blacklisting};
use nym_ecash_contract_common::deposit::{Deposit, DepositId, DepositResponse};
use nym_ecash_contract_common::spend_credential::{
    EcashSpentCredential, EcashSpentCredentialResponse,
};
use nym_validator_client::nym_api::routes::{
    API_VERSION, BANDWIDTH, COCONUT_BLIND_SIGN, COCONUT_ROUTES,
};
use nym_validator_client::nyxd::cosmwasm_client::logs::Log;
use nym_validator_client::nyxd::cosmwasm_client::types::ExecuteResult;
use nym_validator_client::nyxd::{AccountId, ExecTxResult, Fee, Hash, TxResponse};
use rand::rngs::OsRng;
use rand::RngCore;
use rocket::http::Status;
use rocket::local::asynchronous::Client;
use std::collections::{BTreeMap, HashMap};
use std::ops::Deref;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tempfile::{tempdir, TempDir};

pub(crate) mod fixtures;
pub(crate) mod helpers;
mod issued_credentials;

const TEST_COIN_DENOM: &str = "unym";
const TEST_REWARDING_VALIDATOR_ADDRESS: &str = "n19lc9u84cz0yz3fww5283nucc9yvr8gsjmgeul0";

#[derive(Default, Debug)]
struct InternalCounters {
    node_index_counter: NodeIndex,
    tx_hash_counter: u64,
    proposal_id_counter: u64,

    #[allow(dead_code)]
    deposit_id_counter: u32,
}

impl InternalCounters {
    fn next_proposal_id(&mut self) -> NodeIndex {
        self.proposal_id_counter += 1;
        self.proposal_id_counter
    }

    fn next_node_index(&mut self) -> NodeIndex {
        self.node_index_counter += 1;
        self.node_index_counter
    }

    fn next_tx_hash(&mut self) -> Hash {
        use sha2::Digest;

        // just hash the current counter
        self.tx_hash_counter += 1;
        Hash::Sha256(sha2::Sha256::digest(&self.tx_hash_counter.to_be_bytes()).into())
    }

    #[allow(dead_code)]
    fn next_deposit_id(&mut self) -> DepositId {
        self.deposit_id_counter += 1;
        self.deposit_id_counter
    }
}

#[derive(Debug)]
pub(crate) struct Dealing {
    // fake entry is created whenever the metadata is submitted
    pub(crate) metadata: DealingMetadata,
    pub(crate) chunks: BTreeMap<ChunkIndex, PartialContractDealingData>,
}

impl Dealing {
    pub(crate) fn new_metadata_submission(
        dealing_index: DealingIndex,
        chunks: Vec<DealingChunkInfo>,
    ) -> Self {
        Dealing {
            metadata: DealingMetadata::new(dealing_index, chunks),
            chunks: Default::default(),
        }
    }

    pub(crate) fn unchecked_rebuild(&self) -> Vec<u8> {
        let mut data = Vec::new();

        for (chunk_index, partial) in self.chunks.iter() {
            assert!(self
                .metadata
                .submitted_chunks
                .get(chunk_index)
                .unwrap()
                .status
                .submitted());

            data.append(&mut partial.clone())
        }

        data
    }
}

#[derive(Debug)]
pub(crate) struct FakeDkgContractState {
    pub(crate) address: AccountId,

    // pub(crate) dealers: HashMap<NodeIndex, DealerDetails>,
    // pub(crate) past_dealers: HashMap<NodeIndex, DealerDetails>,
    // pub(crate) initial_dealers: Option<InitialReplacementData>,
    pub(crate) dealer_indices: HashMap<String, NodeIndex>,

    // map of epoch id -> dealer -> info
    pub(crate) dealers: HashMap<EpochId, HashMap<String, DealerRegistrationDetails>>,

    // map of epoch id -> dealer -> dealings
    pub(crate) dealings: HashMap<EpochId, HashMap<String, HashMap<DealingIndex, Dealing>>>,

    // map of epoch id -> dealer -> vk share
    pub(crate) verification_shares: HashMap<EpochId, HashMap<String, ContractVKShare>>,

    pub(crate) epoch: Epoch,
    pub(crate) contract_state: ContractState,
    pub(crate) threshold: Option<Threshold>,
}

impl FakeDkgContractState {
    // pub(crate) fn verified_dealers(&self) -> Vec<Addr> {
    //     let epoch_id = self.epoch.epoch_id;
    //     let Some(shares) = self.verification_shares.get(&epoch_id) else {
    //         return Vec::new();
    //     };
    //
    //     shares
    //         .values()
    //         .filter(|s| s.verified)
    //         .map(|s| s.owner.clone())
    //         .collect()
    // }

    fn reset_dkg_state(&mut self) {
        self.threshold = None;
    }

    pub(crate) fn reset_epoch_in_reshare_mode(&mut self) {
        self.reset_dkg_state();
        self.epoch.state = EpochState::PublicKeySubmission { resharing: true };
        self.epoch.epoch_id += 1;
    }

    pub(crate) fn reset_dkg(&mut self) {
        self.reset_dkg_state();
        self.epoch.state = EpochState::PublicKeySubmission { resharing: false };
        self.epoch.epoch_id += 1;
    }

    pub(crate) fn get_registration_details(
        &self,
        addr: &str,
        epoch_id: EpochId,
    ) -> Option<DealerRegistrationDetails> {
        self.dealers.get(&epoch_id)?.get(addr).cloned()
    }

    pub(crate) fn get_dealer_details(
        &self,
        addr: &str,
        epoch_id: EpochId,
    ) -> Option<DealerDetails> {
        let registration_details = self.get_registration_details(addr, epoch_id)?;
        let assigned_index = self.get_dealer_index(addr)?;

        Some(DealerDetails {
            address: Addr::unchecked(addr),
            bte_public_key_with_proof: registration_details.bte_public_key_with_proof,
            ed25519_identity: registration_details.ed25519_identity,
            announce_address: registration_details.announce_address,
            assigned_index,
        })
    }

    // implementation copied from our contract
    pub(crate) fn query_dealer_details(&self, addr: &str) -> DealerDetailsResponse {
        let current_epoch_id = self.epoch.epoch_id;

        // if the address has registration data for the current epoch, it means it's an active dealer
        if let Some(dealer_details) = self.get_dealer_details(addr, current_epoch_id) {
            let assigned_index = dealer_details.assigned_index;
            return DealerDetailsResponse::new(
                Some(dealer_details),
                DealerType::Current { assigned_index },
            );
        }

        // and if has had an assigned index it must have been a dealer at some point in the past
        if let Some(assigned_index) = self.get_dealer_index(addr) {
            return DealerDetailsResponse::new(None, DealerType::Past { assigned_index });
        }

        DealerDetailsResponse::new(None, DealerType::Unknown)
    }

    pub(crate) fn get_dealer_index(&self, addr: &str) -> Option<NodeIndex> {
        self.dealer_indices.get(addr).copied()
    }
}

#[derive(Debug)]
pub(crate) struct FakeGroupContractState {
    pub(crate) address: Addr,
    pub(crate) members: HashMap<String, MemberResponse>,
}

impl FakeGroupContractState {
    pub(crate) fn total_weight(&self) -> u64 {
        self.members
            .values()
            .map(|m| m.weight.unwrap_or_default())
            .sum()
    }

    pub(crate) fn add_member<S: Into<String>>(&mut self, address: S, weight: u64) {
        self.members.insert(
            address.into(),
            MemberResponse {
                weight: Some(weight),
            },
        );
    }
}

#[derive(Debug)]
pub(crate) struct FakeMultisigContractState {
    pub(crate) address: Addr,
    pub(crate) proposals: HashMap<u64, Proposal>,
    pub(crate) votes: HashMap<(String, u64), VoteInfo>,
}

impl FakeMultisigContractState {
    pub(crate) fn reset_votes(&mut self) {
        self.votes = HashMap::new()
    }
}

#[derive(Debug)]
pub(crate) struct FakeBandwidthContractState {
    pub(crate) address: Addr,
    pub(crate) admin: Option<AccountId>,
    pub(crate) spent_credentials: HashMap<String, EcashSpentCredential>,
    pub(crate) deposits: HashMap<DepositId, Deposit>,
    pub(crate) blacklist: HashMap<String, Blacklisting>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SharedFakeChain(Arc<Mutex<FakeChainState>>);

impl Deref for SharedFakeChain {
    type Target = Arc<Mutex<FakeChainState>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub(crate) struct FakeChainState {
    _counters: InternalCounters,

    pub(crate) block_info: BlockInfo,

    pub(crate) txs: HashMap<Hash, TxResponse>,
    pub(crate) dkg_contract: FakeDkgContractState,
    pub(crate) group_contract: FakeGroupContractState,
    pub(crate) multisig_contract: FakeMultisigContractState,
    pub(crate) bandwidth_contract: FakeBandwidthContractState,
}

impl Default for FakeChainState {
    fn default() -> Self {
        let multisig_contract =
            Addr::unchecked("n14ph4e660eyqz0j36zlkaey4zgzexm5twkmjlqaequxr2cjm9eprqsmad6k");
        let group_contract =
            Addr::unchecked("n1pd7kfgvr5tpcv0xnlv46c4jsq9jg2r799xxrcwqdm4l2jhq2pjwqrmz5ju");
        let dkg_contract =
            Addr::unchecked("n1ahg0erc2fs6xx3j5m8sfx3ryuzdjh6kf6qm9plsf865fltekyrfsesac6a");
        let bandwidth_contract =
            Addr::unchecked("n16a32stm6kknhq5cc8rx77elr66pygf2hfszw7wvpq746x3uffylqkjar4l");

        let bandwidth_contract_admin =
            "n1ahg0erc2fs6xx3j5m8sfx3ryuzdjh6kf6qm9plsf865fltekyrfsesac6a"
                .parse()
                .unwrap();

        FakeChainState {
            _counters: Default::default(),

            block_info: mock_env().block,
            txs: HashMap::new(),

            dkg_contract: FakeDkgContractState {
                address: dkg_contract.as_ref().parse().unwrap(),
                dealer_indices: Default::default(),
                dealers: HashMap::new(),

                epoch: Epoch::default(),
                contract_state: ContractState {
                    mix_denom: TEST_COIN_DENOM.to_string(),
                    multisig_addr: multisig_contract.clone(),
                    group_addr: Cw4Contract::new(group_contract.clone()),
                    key_size: 5,
                },
                dealings: HashMap::new(),
                verification_shares: HashMap::new(),
                threshold: None,
            },
            group_contract: FakeGroupContractState {
                address: group_contract,
                members: Default::default(),
            },
            multisig_contract: FakeMultisigContractState {
                address: multisig_contract,
                proposals: Default::default(),
                votes: Default::default(),
            },
            bandwidth_contract: FakeBandwidthContractState {
                address: bandwidth_contract,
                admin: Some(bandwidth_contract_admin),
                spent_credentials: Default::default(),
                deposits: Default::default(),
                blacklist: Default::default(),
            },
        }
    }
}

impl FakeChainState {
    pub(crate) fn get_or_assign_dealer(&mut self, addr: &str) -> NodeIndex {
        if let Some(index) = self.dkg_contract.dealer_indices.get(addr) {
            *index
        } else {
            let new = self._counters.next_node_index();
            self.dkg_contract
                .dealer_indices
                .insert(addr.to_string(), new);
            new
        }
    }

    pub(crate) fn total_group_weight(&self) -> u64 {
        self.group_contract.total_weight()
    }

    pub(crate) fn add_member<S: Into<String>>(&mut self, address: S, weight: u64) {
        self.group_contract.add_member(address, weight)
    }

    pub(crate) fn reset_votes(&mut self) {
        self.multisig_contract.reset_votes()
    }

    pub(crate) fn advance_epoch_in_reshare_mode(&mut self) {
        self.dkg_contract.reset_epoch_in_reshare_mode()
    }

    #[allow(unused)]
    pub(crate) fn advance_epoch_in_reset_mode(&mut self) {
        self.dkg_contract.reset_dkg()
    }

    // TODO: make it return a result
    fn execute_dkg_contract(&mut self, sender: MessageInfo, msg: &Binary) {
        let exec_msg: nym_coconut_dkg_common::msg::ExecuteMsg = from_binary(msg).unwrap();
        match exec_msg {
            nym_coconut_dkg_common::msg::ExecuteMsg::VerifyVerificationKeyShare {
                owner,
                resharing,
            } => {
                if sender.sender != self.multisig_contract.address {
                    panic!("not multisig")
                }
                assert_eq!(
                    self.dkg_contract.epoch.state,
                    EpochState::VerificationKeyFinalization { resharing }
                );
                let epoch_id = self.dkg_contract.epoch.epoch_id;
                let Some(shares) = self.dkg_contract.verification_shares.get_mut(&epoch_id) else {
                    unimplemented!("no shares for epoch")
                };
                let Some(share) = shares.get_mut(owner.as_str()) else {
                    unimplemented!("no shares for owner")
                };
                share.verified = true
            }
            other => unimplemented!("unimplemented exec of {other:?}"),
        }
    }

    // TODO: make it return a result
    fn execute_contract_msg(&mut self, contract: &String, msg: &Binary, sender: MessageInfo) {
        if contract == &self.group_contract.address {
            unimplemented!("group contract exec")
        }
        if contract == &self.multisig_contract.address {
            unimplemented!("multisig contract exec")
        }
        if contract == &self.bandwidth_contract.address {
            unimplemented!("bandwidth contract exec")
        }
        if contract == self.dkg_contract.address.as_ref() {
            return self.execute_dkg_contract(sender, msg);
        }
        panic!("unknown contract {contract}")
    }

    // TODO: make it return a result
    fn execute_wasm_msg(&mut self, msg: &WasmMsg, sender_address: Addr) {
        match msg {
            WasmMsg::Execute {
                contract_addr,
                msg,
                funds,
            } => {
                let sender = mock_info(sender_address.as_ref(), funds);
                self.execute_contract_msg(contract_addr, msg, sender)
            }
            other => unimplemented!("unimplemented wasm proposal for {other:?}"),
        }
    }

    // TODO: make it return a result
    pub(crate) fn execute_msg(&mut self, msg: &CosmosMsg, sender_address: AccountId) {
        match msg {
            CosmosMsg::Wasm(wasm_msg) => {
                self.execute_wasm_msg(wasm_msg, Addr::unchecked(sender_address.as_ref()))
            }
            other => unimplemented!("unimplemented proposal for {other:?}"),
        };
    }
}

fn proposal_to_response(
    proposal_id: u64,
    block: &BlockInfo,
    proposal: Proposal,
) -> ProposalResponse {
    // replicate behaviour from `query_proposal` of cw3
    let status = proposal.current_status(block);
    let threshold = proposal.threshold.to_response(proposal.total_weight);
    ProposalResponse {
        id: proposal_id,
        title: proposal.title,
        description: proposal.description,
        msgs: proposal.msgs,
        status,
        expires: proposal.expires,
        threshold,
        proposer: proposal.proposer,
        deposit: proposal.deposit,
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DummyClient {
    validator_address: AccountId,

    state: SharedFakeChain,
}

impl DummyClient {
    pub fn new(validator_address: AccountId, state: SharedFakeChain) -> Self {
        Self {
            validator_address,
            state,
        }
    }

    #[allow(dead_code)]
    pub fn chain_state(&self) -> SharedFakeChain {
        self.state.clone()
    }
}

#[async_trait]
impl super::client::Client for DummyClient {
    async fn address(&self) -> AccountId {
        self.validator_address.clone()
    }

    async fn dkg_contract_address(&self) -> Result<AccountId> {
        Ok(self.state.lock().unwrap().dkg_contract.address.clone())
    }

    async fn bandwidth_contract_admin(&self) -> Result<Option<AccountId>> {
        Ok(self.state.lock().unwrap().bandwidth_contract.admin.clone())
    }

    async fn get_deposit(&self, deposit_id: DepositId) -> Result<DepositResponse> {
        let deposit = self
            .state
            .lock()
            .unwrap()
            .bandwidth_contract
            .deposits
            .get(&deposit_id)
            .cloned();

        Ok(DepositResponse {
            id: deposit_id,
            deposit,
        })
    }

    async fn get_proposal(&self, proposal_id: u64) -> Result<ProposalResponse> {
        let chain = self.state.lock().unwrap();
        let proposal = chain
            .multisig_contract
            .proposals
            .get(&proposal_id)
            .cloned()
            .ok_or(CoconutError::IncorrectProposal {
                reason: String::from("proposal not found"),
            })?;

        // replicate behaviour from `query_proposal` of cw3
        Ok(proposal_to_response(
            proposal_id,
            &chain.block_info,
            proposal,
        ))
    }

    async fn list_proposals(&self) -> Result<Vec<ProposalResponse>> {
        let chain = self.state.lock().unwrap();
        let block = &chain.block_info;

        Ok(chain
            .multisig_contract
            .proposals
            .iter()
            .map(|(id, proposal)| proposal_to_response(*id, block, proposal.clone()))
            .collect())
    }

    async fn get_vote(&self, proposal_id: u64, voter: String) -> Result<VoteResponse> {
        let vote = self
            .state
            .lock()
            .unwrap()
            .multisig_contract
            .votes
            .get(&(voter, proposal_id))
            .cloned();

        Ok(VoteResponse { vote })
    }

    async fn get_spent_credential(
        &self,
        blinded_serial_number: String,
    ) -> Result<EcashSpentCredentialResponse> {
        Ok(EcashSpentCredentialResponse::new(
            self.state
                .lock()
                .unwrap()
                .bandwidth_contract
                .spent_credentials
                .get(&blinded_serial_number)
                .cloned(),
        ))
    }

    async fn propose_for_blacklist(&self, _public_key: String) -> Result<ExecuteResult> {
        todo!()
    }

    async fn get_blacklisted_account(
        &self,
        public_key: String,
    ) -> Result<BlacklistedAccountResponse> {
        Ok(BlacklistedAccountResponse::new(
            self.state
                .lock()
                .unwrap()
                .bandwidth_contract
                .blacklist
                .get(&public_key)
                .cloned(),
        ))
    }

    async fn contract_state(&self) -> Result<ContractState> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .dkg_contract
            .contract_state
            .clone())
    }

    async fn get_current_epoch(&self) -> Result<Epoch> {
        Ok(self.state.lock().unwrap().dkg_contract.epoch)
    }

    async fn group_member(&self, addr: String) -> Result<MemberResponse> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .group_contract
            .members
            .get(&addr)
            .cloned()
            .unwrap_or(MemberResponse { weight: None }))
    }

    async fn get_current_epoch_threshold(&self) -> Result<Option<Threshold>> {
        Ok(self.state.lock().unwrap().dkg_contract.threshold)
    }

    async fn get_self_registered_dealer_details(&self) -> Result<DealerDetailsResponse> {
        let address = self.validator_address.as_ref();
        Ok(self
            .state
            .lock()
            .unwrap()
            .dkg_contract
            .query_dealer_details(address))
    }

    async fn get_registered_dealer_details(
        &self,
        epoch_id: EpochId,
        dealer: String,
    ) -> Result<RegisteredDealerDetails> {
        let details = self
            .state
            .lock()
            .unwrap()
            .dkg_contract
            .dealers
            .get(&epoch_id)
            .and_then(|dealers| dealers.get(&dealer))
            .cloned();
        Ok(RegisteredDealerDetails { details })
    }

    async fn get_dealer_dealings_status(
        &self,
        epoch_id: EpochId,
        dealer: String,
    ) -> Result<DealerDealingsStatusResponse> {
        let guard = self.state.lock().unwrap();
        let key_size = guard.dkg_contract.contract_state.key_size;

        let dealer_addr = Addr::unchecked(&dealer);

        let Some(epoch_dealings) = guard.dkg_contract.dealings.get(&epoch_id) else {
            return Ok(DealerDealingsStatusResponse {
                epoch_id,
                dealer: dealer_addr,
                all_dealings_fully_submitted: false,
                dealing_submission_status: Default::default(),
            });
        };

        let Some(dealer_dealings) = epoch_dealings.get(&dealer) else {
            return Ok(DealerDealingsStatusResponse {
                epoch_id,
                dealer: dealer_addr,
                all_dealings_fully_submitted: false,
                dealing_submission_status: Default::default(),
            });
        };

        let mut dealing_submission_status: BTreeMap<DealingIndex, DealingStatus> = BTreeMap::new();
        for dealing_index in 0..key_size {
            let metadata = dealer_dealings
                .get(&dealing_index)
                .map(|d| d.metadata.clone());
            dealing_submission_status.insert(dealing_index, metadata.into());
        }

        Ok(DealerDealingsStatusResponse {
            epoch_id,
            dealer: Addr::unchecked(&dealer),
            all_dealings_fully_submitted: dealing_submission_status
                .values()
                .all(|d| d.fully_submitted),
            dealing_submission_status,
        })
    }

    async fn get_dealing_status(
        &self,
        epoch_id: EpochId,
        dealer: String,
        dealing_index: DealingIndex,
    ) -> Result<DealingStatusResponse> {
        let guard = self.state.lock().unwrap();

        let metadata = guard
            .dkg_contract
            .dealings
            .get(&epoch_id)
            .and_then(|epoch_dealings| epoch_dealings.get(&dealer))
            .and_then(|dealer_dealings| dealer_dealings.get(&dealing_index))
            .map(|info| info.metadata.clone());

        Ok(DealingStatusResponse {
            epoch_id,
            dealer: Addr::unchecked(dealer),
            dealing_index,
            status: metadata.into(),
        })
    }

    async fn get_current_dealers(&self) -> Result<Vec<DealerDetails>> {
        let chain = self.state.lock().unwrap();
        let current_epoch_id = chain.dkg_contract.epoch.epoch_id;

        let Some(epoch_dealers) = chain.dkg_contract.dealers.get(&current_epoch_id) else {
            return Ok(Vec::new());
        };

        Ok(epoch_dealers
            .iter()
            .map(|(address, details)| {
                let assigned_index = chain.dkg_contract.get_dealer_index(address).unwrap();
                DealerDetails {
                    address: Addr::unchecked(address),
                    bte_public_key_with_proof: details.bte_public_key_with_proof.clone(),
                    ed25519_identity: details.ed25519_identity.clone(),
                    announce_address: details.announce_address.clone(),
                    assigned_index,
                }
            })
            .collect())
    }

    async fn get_dealing_metadata(
        &self,
        epoch_id: EpochId,
        dealer: String,
        dealing_index: DealingIndex,
    ) -> Result<Option<DealingMetadata>> {
        let guard = self.state.lock().unwrap();

        let Some(epoch_dealings) = guard.dkg_contract.dealings.get(&epoch_id) else {
            return Ok(None);
        };

        let Some(dealer_dealings) = epoch_dealings.get(&dealer) else {
            return Ok(None);
        };

        let Some(dealing) = dealer_dealings.get(&dealing_index) else {
            return Ok(None);
        };

        Ok(Some(dealing.metadata.clone()))
    }

    async fn get_dealing_chunk(
        &self,
        epoch_id: EpochId,
        dealer: &str,
        dealing_index: DealingIndex,
        chunk_index: ChunkIndex,
    ) -> Result<Option<PartialContractDealingData>> {
        let guard = self.state.lock().unwrap();

        let Some(epoch_dealings) = guard.dkg_contract.dealings.get(&epoch_id) else {
            return Ok(None);
        };

        let Some(dealer_dealings) = epoch_dealings.get(dealer) else {
            return Ok(None);
        };

        let Some(dealing) = dealer_dealings.get(&dealing_index) else {
            return Ok(None);
        };

        Ok(dealing.chunks.get(&chunk_index).cloned())
    }

    async fn get_verification_key_share(
        &self,
        epoch_id: EpochId,
        dealer: String,
    ) -> Result<Option<ContractVKShare>> {
        let guard = self.state.lock().unwrap();
        let epoch_shares = guard.dkg_contract.verification_shares.get(&epoch_id);

        match epoch_shares {
            None => Ok(None),
            Some(epoch_shares) => Ok(epoch_shares.get(&dealer).cloned()),
        }
    }

    async fn get_verification_key_shares(&self, epoch_id: EpochId) -> Result<Vec<ContractVKShare>> {
        let guard = self.state.lock().unwrap();
        let epoch_shares = guard.dkg_contract.verification_shares.get(&epoch_id);

        match epoch_shares {
            None => Ok(Vec::new()),
            Some(epoch_shares) => Ok(epoch_shares.values().cloned().collect()),
        }
    }

    async fn vote_proposal(
        &self,
        proposal_id: u64,
        vote_yes: bool,
        _fee: Option<Fee>,
    ) -> Result<()> {
        let voter = self.validator_address.to_string();
        let mut chain = self.state.lock().unwrap();
        if !chain.multisig_contract.proposals.contains_key(&proposal_id) {
            return Err(CoconutError::IncorrectProposal {
                reason: String::from("proposal not found"),
            });
        }

        // for now we assume every group member is a voter
        let weight = chain
            .group_contract
            .members
            .get(&voter)
            .expect("todo: not a voter")
            .weight
            .expect("no vote weight");

        let vote = if vote_yes { Vote::Yes } else { Vote::No };

        if chain
            .multisig_contract
            .votes
            .contains_key(&(voter.clone(), proposal_id))
        {
            todo!("already voted");
        }
        chain.multisig_contract.votes.insert(
            (voter.clone(), proposal_id),
            VoteInfo {
                proposal_id,
                voter,
                vote,
                weight,
            },
        );

        let block_info = chain.block_info.clone();
        if let Some(proposal) = chain.multisig_contract.proposals.get_mut(&proposal_id) {
            proposal.votes.add_vote(vote, weight);
            proposal.update_status(&block_info)
        }

        Ok(())
    }
    async fn execute_proposal(&self, proposal_id: u64) -> Result<()> {
        let mut chain = self.state.lock().unwrap();
        let multisig_address: AccountId = chain.multisig_contract.address.as_str().parse().unwrap();

        let Some(proposal) = chain.multisig_contract.proposals.get_mut(&proposal_id) else {
            return Err(CoconutError::ProposalIdError {
                reason: String::from("proposal id not found"),
            });
        };

        if proposal.status != cw3::Status::Passed {
            unimplemented!("proposal hasn't been passed")
        }
        proposal.status = cw3::Status::Executed;

        for msg in &proposal.msgs.clone() {
            chain.execute_msg(msg, multisig_address.clone());
        }

        Ok(())
    }

    async fn can_advance_epoch_state(&self) -> Result<bool> {
        // TODO: incorporate the short-circuiting logic in here
        let chain = self.state.lock().unwrap();
        let epoch = chain.dkg_contract.epoch;
        Ok(if let Some(finish_timestamp) = epoch.deadline {
            finish_timestamp <= chain.block_info.time
        } else {
            false
        })
    }

    async fn advance_epoch_state(&self) -> Result<()> {
        todo!()
    }

    async fn register_dealer(
        &self,
        bte_public_key_with_proof: EncodedBTEPublicKeyWithProof,
        identity_key: IdentityKey,
        announce_address: String,
        _resharing: bool,
    ) -> Result<ExecuteResult> {
        let mut guard = self.state.lock().unwrap();
        let assigned_index = guard.get_or_assign_dealer(self.validator_address.as_ref());
        let epoch = guard.dkg_contract.epoch.epoch_id;

        let dealer_details = DealerRegistrationDetails {
            bte_public_key_with_proof,
            ed25519_identity: identity_key,
            announce_address,
        };

        let epoch_dealers = guard.dkg_contract.dealers.entry(epoch).or_default();
        if !epoch_dealers.contains_key(self.validator_address.as_ref()) {
            epoch_dealers.insert(self.validator_address.to_string(), dealer_details);
        } else {
            unimplemented!("already registered")
        }

        let transaction_hash = guard._counters.next_tx_hash();

        Ok(ExecuteResult {
            logs: vec![Log {
                msg_index: 0,
                events: vec![cosmwasm_std::Event::new("wasm")
                    .add_attribute(NODE_INDEX, assigned_index.to_string())],
            }],
            data: Default::default(),
            transaction_hash,
            gas_info: Default::default(),
        })
    }

    async fn submit_dealing_metadata(
        &self,
        dealing_index: DealingIndex,
        chunks: Vec<DealingChunkInfo>,
        _resharing: bool,
    ) -> Result<ExecuteResult> {
        let mut guard = self.state.lock().unwrap();
        let current_epoch = guard.dkg_contract.epoch.epoch_id;

        let epoch_dealings = guard
            .dkg_contract
            .dealings
            .entry(current_epoch)
            .or_default();

        let dealer_dealings = epoch_dealings
            .entry(self.validator_address.to_string())
            .or_default();
        dealer_dealings.insert(
            dealing_index,
            Dealing::new_metadata_submission(dealing_index, chunks),
        );

        let transaction_hash = guard._counters.next_tx_hash();

        Ok(ExecuteResult {
            logs: vec![],
            data: Default::default(),
            transaction_hash,
            gas_info: Default::default(),
        })
    }

    async fn submit_dealing_chunk(&self, chunk: PartialContractDealing) -> Result<ExecuteResult> {
        let mut guard = self.state.lock().unwrap();
        let current_epoch = guard.dkg_contract.epoch.epoch_id;
        let current_height = guard.block_info.height;

        // normally we should do checks for existence, etc.
        // but since this is a testing code, we assume everything is sent in order and the appropriate entries exist
        let epoch_dealings = guard.dkg_contract.dealings.get_mut(&current_epoch).unwrap();

        let dealer_dealings = epoch_dealings
            .get_mut(self.validator_address.as_ref())
            .unwrap();

        let dealing_chunks = dealer_dealings.get_mut(&chunk.dealing_index).unwrap();
        dealing_chunks.chunks.insert(chunk.chunk_index, chunk.data);

        dealing_chunks
            .metadata
            .submitted_chunks
            .get_mut(&chunk.chunk_index)
            .unwrap()
            .status
            .submission_height = Some(current_height);

        let transaction_hash = guard._counters.next_tx_hash();

        Ok(ExecuteResult {
            logs: vec![],
            data: Default::default(),
            transaction_hash,
            gas_info: Default::default(),
        })
    }

    async fn submit_verification_key_share(
        &self,
        share: VerificationKeyShare,
        resharing: bool,
    ) -> Result<ExecuteResult> {
        let mut chain = self.state.lock().unwrap();

        let address = self.validator_address.to_string();
        let epoch_id = chain.dkg_contract.epoch.epoch_id;
        let Some(dealer_details) = chain.dkg_contract.get_dealer_details(&address, epoch_id) else {
            // Just throw some error, not really the correct one
            return Err(CoconutError::DepositInfoNotFound);
        };

        let dkg_contract = chain.dkg_contract.address.clone();

        chain
            .dkg_contract
            .verification_shares
            .entry(epoch_id)
            .or_default()
            .insert(
                self.validator_address.to_string(),
                ContractVKShare {
                    share,
                    announce_address: dealer_details.announce_address.clone(),
                    node_index: dealer_details.assigned_index,
                    owner: Addr::unchecked(&address),
                    epoch_id,
                    verified: false,
                },
            );

        let proposal_id = chain._counters.next_proposal_id();
        let verify_vk_share_req =
            nym_coconut_dkg_common::msg::ExecuteMsg::VerifyVerificationKeyShare {
                owner: address,
                resharing,
            };
        let verify_vk_share_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: chain.dkg_contract.address.to_string(),
            msg: to_binary(&verify_vk_share_req).unwrap(),
            funds: vec![],
        });
        let proposal = Proposal {
            title: String::new(),
            description: String::new(),
            msgs: vec![verify_vk_share_msg],
            status: cw3::Status::Open,
            expires: cw_utils::Expiration::Never {},
            threshold: cw_utils::Threshold::AbsolutePercentage {
                percentage: Decimal::from_ratio(2u32, 3u32),
            },
            total_weight: chain.total_group_weight(),
            votes: Votes::yes(0),
            proposer: Addr::unchecked(dkg_contract.as_ref()),
            deposit: None,
            start_height: 0,
        };
        chain
            .multisig_contract
            .proposals
            .insert(proposal_id, proposal);
        let transaction_hash = chain._counters.next_tx_hash();
        Ok(ExecuteResult {
            logs: vec![Log {
                msg_index: 0,
                events: vec![cosmwasm_std::Event::new("wasm")
                    .add_attribute(DKG_PROPOSAL_ID, proposal_id.to_string())],
            }],
            data: Default::default(),
            transaction_hash,
            gas_info: Default::default(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct DummyCommunicationChannel {
    current_epoch: Arc<AtomicU64>,
    aggregated_verification_key: VerificationKeyAuth,
}

impl DummyCommunicationChannel {
    pub fn new(aggregated_verification_key: VerificationKeyAuth) -> Self {
        DummyCommunicationChannel {
            current_epoch: Arc::new(AtomicU64::new(1)),
            aggregated_verification_key,
        }
    }

    pub fn with_epoch(mut self, current_epoch: Arc<AtomicU64>) -> Self {
        self.current_epoch = current_epoch;
        self
    }
}

#[async_trait]
impl super::comm::APICommunicationChannel for DummyCommunicationChannel {
    async fn current_epoch(&self) -> Result<EpochId> {
        Ok(self.current_epoch.load(Ordering::Relaxed))
    }

    async fn aggregated_verification_key(&self, _epoch_id: EpochId) -> Result<VerificationKeyAuth> {
        Ok(self.aggregated_verification_key.clone())
    }
}

#[allow(dead_code)]
pub fn deposit_fixture() -> Deposit {
    let mut rng = OsRng;
    let identity_keypair = identity::KeyPair::new(&mut rng);

    Deposit {
        info: CredentialType::TicketBook.to_string(),
        amount: 1234,
        bs58_encoded_ed25519: identity_keypair.public_key().to_base58_string(),
    }
}

#[allow(dead_code)]
pub fn tx_entry_fixture(hash: Hash) -> TxResponse {
    TxResponse {
        hash,
        height: Default::default(),
        index: 0,
        tx_result: ExecTxResult {
            code: Default::default(),
            data: Default::default(),
            log: Default::default(),
            info: Default::default(),
            gas_wanted: Default::default(),
            gas_used: Default::default(),
            events: vec![],
            codespace: Default::default(),
        },
        tx: vec![],
        proof: None,
    }
}

pub fn blinded_signature_fixture() -> BlindedSignature {
    let gen1_bytes = [
        151u8, 241, 211, 167, 49, 151, 215, 148, 38, 149, 99, 140, 79, 169, 172, 15, 195, 104, 140,
        79, 151, 116, 185, 5, 161, 78, 58, 63, 23, 27, 172, 88, 108, 85, 232, 63, 249, 122, 26,
        239, 251, 58, 240, 10, 219, 34, 198, 187,
    ];

    let dummy_bytes = gen1_bytes
        .iter()
        .chain(gen1_bytes.iter())
        .copied()
        .collect::<Vec<_>>();

    BlindedSignature::from_bytes(&dummy_bytes).unwrap()
}

pub fn voucher_fixture(deposit_id: Option<DepositId>) -> IssuanceBandwidthCredential {
    let mut rng = OsRng;
    let deposit_id = deposit_id.unwrap_or(69);

    let identity_keypair = identity::KeyPair::new(&mut rng);

    let id_priv =
        identity::PrivateKey::from_bytes(&identity_keypair.private_key().to_bytes()).unwrap();
    let identifier = [44u8; 32];
    // (voucher, request)
    IssuanceBandwidthCredential::new_voucher(deposit_id, &identifier, id_priv)
}

fn dummy_signature() -> identity::Signature {
    "3vUCc6MCN5AC2LNgDYjRB1QeErZSN1S8f6K14JHjpUcKWXbjGYFExA8DbwQQBki9gyUqrpBF94Drttb4eMcGQXkp"
        .parse()
        .unwrap()
}

struct TestFixture {
    rocket: Client,
    storage: NymApiStorage,
    chain_state: SharedFakeChain,
    epoch: Arc<AtomicU64>,

    _tmp_dir: TempDir,
}

impl TestFixture {
    async fn new() -> Self {
        let mut rng = crate::ecash::tests::fixtures::test_rng([69u8; 32]);
        let coconut_keypair = ttp_keygen(1, 1).unwrap().remove(0);
        let identity = identity::KeyPair::new(&mut rng);
        let epoch = Arc::new(AtomicU64::new(1));
        let comm_channel =
            DummyCommunicationChannel::new(coconut_keypair.verification_key().clone())
                .with_epoch(epoch.clone());

        // TODO: it's AWFUL to test with actual storage, we should somehow abstract it away
        let tmp_dir = tempdir().unwrap();
        let storage = NymApiStorage::init(tmp_dir.path().join("TESTING_STORAGE.db"))
            .await
            .unwrap();

        let staged_key_pair = crate::ecash::KeyPair::new();
        staged_key_pair
            .set(KeyPairWithEpoch {
                keys: coconut_keypair,
                issued_for_epoch: 1,
            })
            .await;
        staged_key_pair.validate();

        let chain_state = SharedFakeChain::default();
        let nyxd_client = DummyClient::new(
            AccountId::from_str(TEST_REWARDING_VALIDATOR_ADDRESS).unwrap(),
            chain_state.clone(),
        );

        let rocket = rocket::build().attach(crate::ecash::stage(
            nyxd_client,
            identity,
            staged_key_pair,
            comm_channel,
            storage.clone(),
        ));

        TestFixture {
            rocket: Client::tracked(rocket)
                .await
                .expect("valid rocket instance"),
            storage,
            chain_state,
            epoch,
            _tmp_dir: tmp_dir,
        }
    }

    fn set_epoch(&self, epoch: u64) {
        self.epoch.store(epoch, Ordering::Relaxed)
    }

    #[allow(dead_code)]
    fn add_tx(&self, hash: Hash, tx: TxResponse) {
        self.chain_state.lock().unwrap().txs.insert(hash, tx);
    }

    fn add_deposit(&self, voucher_data: &BandwidthVoucherIssuanceData) {
        let mut chain = self.chain_state.lock().unwrap();
        let deposit = Deposit {
            info: CredentialType::TicketBook.to_string(),
            amount: voucher_data.value(),
            bs58_encoded_ed25519: voucher_data.identity_key().public_key().to_base58_string(),
        };
        let existing = chain
            .bandwidth_contract
            .deposits
            .insert(voucher_data.deposit_id(), deposit);
        assert!(existing.is_none());
    }

    async fn issue_dummy_credential(&self) {
        let mut rng = OsRng;
        let deposit_id = rng.next_u32();

        let voucher = voucher_fixture(Some(deposit_id));

        let signing_data = voucher.prepare_for_signing();
        let voucher_data = voucher.get_variant_data().voucher_data().unwrap();
        let req = voucher_data.create_blind_sign_request_body(&signing_data);

        self.add_deposit(voucher_data);
        self.issue_credential(req).await;
    }

    async fn issue_credential(&self, req: BlindSignRequestBody) -> BlindedSignatureResponse {
        let response = self
            .rocket
            .post(format!(
                "/{API_VERSION}/{COCONUT_ROUTES}/{BANDWIDTH}/{COCONUT_BLIND_SIGN}",
            ))
            .json(&req)
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        serde_json::from_str(&response.into_string().await.unwrap()).unwrap()
    }

    async fn issued_credential(&self, id: i64) -> Option<IssuedCredentialResponse> {
        let response = self
            .rocket
            .get(format!(
                "/{API_VERSION}/{COCONUT_ROUTES}/{BANDWIDTH}/issued-credential/{id}"
            ))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        serde_json::from_str(&response.into_string().await.unwrap()).unwrap()
    }

    async fn issued_unchecked(&self, id: i64) -> IssuedCredentialBody {
        self.issued_credential(id)
            .await
            .unwrap()
            .credential
            .unwrap()
    }
}

#[cfg(test)]
mod credential_tests {
    use super::*;
    use crate::ecash::tests::helpers::init_chain;
    use nym_api_requests::coconut::{
        models::VerifyEcashCredentialResponse, VerifyEcashCredentialBody,
    };
    use nym_compact_ecash::{
        ecash_parameters, issue,
        tests::helpers::{generate_coin_indices_signatures, generate_expiration_date_signatures},
        ttp_keygen, PayInfo,
    };
    use nym_validator_client::nym_api::routes::ECASH_VERIFY_ONLINE_CREDENTIAL;

    #[tokio::test]
    async fn already_issued() {
        let voucher = voucher_fixture(None);
        let signing_data = voucher.prepare_for_signing();
        let voucher_data = voucher.get_variant_data().voucher_data().unwrap();
        let request_body = voucher_data.create_blind_sign_request_body(&signing_data);

        let deposit_id = request_body.deposit_id;

        let test_fixture = TestFixture::new().await;
        test_fixture.add_deposit(voucher_data);

        let sig = blinded_signature_fixture();
        let commitments = request_body.encode_commitments();
        let expiration_date = request_body.expiration_date;
        test_fixture
            .storage
            .store_issued_credential(
                42,
                deposit_id,
                &sig,
                dummy_signature(),
                commitments,
                expiration_date,
            )
            .await
            .unwrap();

        let response = test_fixture
            .rocket
            .post(format!(
                "/{}/{}/{}/{}",
                API_VERSION, COCONUT_ROUTES, BANDWIDTH, COCONUT_BLIND_SIGN
            ))
            .json(&request_body)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let expected_response = BlindedSignatureResponse::new(sig);

        // This is a more direct way, but there's a bug which makes it hang https://github.com/SergioBenitez/Rocket/issues/1893
        // let blinded_signature_response = response
        //     .into_json::<BlindedSignatureResponse>()
        //     .await
        //     .unwrap();
        let blinded_signature_response = serde_json::from_str::<BlindedSignatureResponse>(
            &response.into_string().await.unwrap(),
        )
        .unwrap();
        assert_eq!(
            blinded_signature_response.to_bytes(),
            expected_response.to_bytes()
        );
    }

    #[tokio::test]
    async fn state_functions() {
        let mut rng = OsRng;
        let identity = identity::KeyPair::new(&mut rng);

        let nyxd_client = DummyClient::new(
            AccountId::from_str(TEST_REWARDING_VALIDATOR_ADDRESS).unwrap(),
            Default::default(),
        );
        let key_pair = ttp_keygen(1, 1).unwrap().remove(0);
        let tmp_dir = tempdir().unwrap();

        let storage = NymApiStorage::init(tmp_dir.path().join("storage.db"))
            .await
            .unwrap();
        let comm_channel = DummyCommunicationChannel::new(key_pair.verification_key().clone());
        let staged_key_pair = crate::ecash::KeyPair::new();
        staged_key_pair
            .set(KeyPairWithEpoch {
                keys: key_pair,
                issued_for_epoch: 1,
            })
            .await;
        staged_key_pair.validate();

        let state = State::new(
            nyxd_client,
            identity,
            staged_key_pair,
            comm_channel,
            storage.clone(),
        );

        let deposit_id = 42;
        assert!(state.already_issued(deposit_id).await.unwrap().is_none());

        let voucher = voucher_fixture(None);
        let signing_data = voucher.prepare_for_signing();
        let voucher_data = voucher.get_variant_data().voucher_data().unwrap();
        let request_body = voucher_data.create_blind_sign_request_body(&signing_data);

        let commitments = request_body.encode_commitments();
        let expiration_date = request_body.expiration_date;
        let sig = blinded_signature_fixture();
        storage
            .store_issued_credential(
                42,
                deposit_id,
                &sig,
                dummy_signature(),
                commitments.clone(),
                expiration_date,
            )
            .await
            .unwrap();

        assert_eq!(
            state
                .already_issued(deposit_id)
                .await
                .unwrap()
                .unwrap()
                .to_bytes(),
            blinded_signature_fixture().to_bytes()
        );

        let blinded_signature = BlindedSignature::from_bytes(&[
            183, 217, 166, 113, 40, 123, 74, 25, 72, 31, 136, 19, 125, 95, 217, 228, 96, 113, 25,
            240, 12, 102, 125, 11, 174, 20, 216, 82, 192, 71, 27, 194, 48, 20, 17, 95, 243, 179,
            82, 21, 57, 143, 101, 19, 22, 186, 147, 13, 147, 238, 39, 119, 15, 36, 251, 131, 250,
            38, 185, 113, 187, 40, 227, 107, 134, 190, 123, 183, 126, 176, 226, 173, 147, 137, 17,
            175, 13, 115, 78, 222, 119, 93, 146, 116, 229, 0, 152, 51, 232, 2, 102, 204, 147, 202,
            254, 243,
        ])
        .unwrap();

        // Check that the new payload is not stored if there was already something signed for tx_hash
        let storage_err = storage
            .store_issued_credential(
                42,
                deposit_id,
                &blinded_signature,
                dummy_signature(),
                commitments.clone(),
                expiration_date,
            )
            .await;
        assert!(storage_err.is_err());

        // And use a new deposit to store a new signature
        let deposit_id = 69;

        storage
            .store_issued_credential(
                42,
                deposit_id,
                &blinded_signature,
                dummy_signature(),
                commitments.clone(),
                expiration_date,
            )
            .await
            .unwrap();

        // Check that the same value for tx_hash is returned
        assert_eq!(
            state
                .already_issued(deposit_id)
                .await
                .unwrap()
                .unwrap()
                .to_bytes(),
            blinded_signature.to_bytes()
        );
    }

    #[tokio::test]
    async fn blind_sign_correct() {
        let deposit_id = 42;

        let mut rng = OsRng;
        let nym_api_identity = identity::KeyPair::new(&mut rng);

        let identity_keypair = identity::KeyPair::new(&mut rng);
        let identifier = [42u8; 32];
        let voucher = IssuanceBandwidthCredential::new_voucher(
            deposit_id,
            &identifier,
            identity::PrivateKey::from_base58_string(
                identity_keypair.private_key().to_base58_string(),
            )
            .unwrap(),
        );

        let key_pair = ttp_keygen(1, 1).unwrap().remove(0);
        let tmp_dir = tempdir().unwrap();
        let storage = NymApiStorage::init(tmp_dir.path().join("storage.db"))
            .await
            .unwrap();

        let chain = init_chain();
        let voucher_data = voucher.get_variant_data().voucher_data().unwrap();

        let deposit = Deposit {
            info: CredentialType::TicketBook.to_string(),
            amount: voucher_data.value(),
            bs58_encoded_ed25519: voucher_data.identity_key().public_key().to_base58_string(),
        };
        chain
            .lock()
            .unwrap()
            .bandwidth_contract
            .deposits
            .insert(voucher_data.deposit_id(), deposit);

        let nyxd_client = DummyClient::new(
            AccountId::from_str(TEST_REWARDING_VALIDATOR_ADDRESS).unwrap(),
            chain.clone(),
        );

        let comm_channel = DummyCommunicationChannel::new(key_pair.verification_key().clone());
        let staged_key_pair = crate::ecash::KeyPair::new();
        staged_key_pair
            .set(KeyPairWithEpoch {
                keys: key_pair,
                issued_for_epoch: 1,
            })
            .await;
        staged_key_pair.validate();

        let rocket = rocket::build().attach(crate::ecash::stage(
            nyxd_client,
            nym_api_identity,
            staged_key_pair,
            comm_channel,
            storage.clone(),
        ));
        let client = Client::tracked(rocket)
            .await
            .expect("valid rocket instance");

        let signing_data = voucher.prepare_for_signing();
        let voucher_data = voucher.get_variant_data().voucher_data().unwrap();
        let request_body = voucher_data.create_blind_sign_request_body(&signing_data);

        let response = client
            .post(format!(
                "/{}/{}/{}/{}",
                API_VERSION, COCONUT_ROUTES, BANDWIDTH, COCONUT_BLIND_SIGN
            ))
            .json(&request_body)
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        // This is a more direct way, but there's a bug which makes it hang https://github.com/SergioBenitez/Rocket/issues/1893
        // assert!(response.into_json::<BlindedSignatureResponse>().is_some());
        let blinded_signature_response = serde_json::from_str::<BlindedSignatureResponse>(
            &response.into_string().await.unwrap(),
        );
        assert!(blinded_signature_response.is_ok());
    }

    #[tokio::test]
    async fn verification_of_bandwidth_credential() {
        // Setup variables
        let chain = init_chain();
        let validator_address = AccountId::from_str(TEST_REWARDING_VALIDATOR_ADDRESS).unwrap();
        chain
            .lock()
            .unwrap()
            .add_member(validator_address.as_ref(), 100);

        let nyxd_client = DummyClient::new(validator_address.clone(), chain.clone());
        let db_dir = tempdir().unwrap();

        // generate all the credential requests
        let key_pair = ttp_keygen(1, 1).unwrap().remove(0);
        let epoch = 1;

        let issuance = voucher_fixture(None);
        let sig_req = issuance.prepare_for_signing();
        let exp_date_sigs = generate_expiration_date_signatures(
            sig_req.expiration_date.unix_timestamp() as u64,
            &[key_pair.secret_key()],
            &vec![key_pair.verification_key()],
            &key_pair.verification_key(),
            &[key_pair.index.unwrap()],
        )
        .unwrap();

        let blind_sig = issue(
            key_pair.secret_key(),
            sig_req.ecash_pub_key.clone(),
            &sig_req.withdrawal_request,
            sig_req.expiration_date.unix_timestamp() as u64,
        )
        .unwrap();
        let partial_wallet = issuance
            .unblind_signature(
                &key_pair.verification_key(),
                &sig_req,
                blind_sig,
                key_pair.index.unwrap(),
            )
            .unwrap();

        let wallet = issuance
            .aggregate_signature_shares(
                &key_pair.verification_key(),
                &vec![partial_wallet],
                sig_req,
            )
            .unwrap();

        let mut issued =
            issuance.to_issued_credential(wallet.clone(), exp_date_sigs.clone(), epoch);
        let mut issued2 = issuance.to_issued_credential(wallet, exp_date_sigs, epoch);

        let coin_indices_signatures = generate_coin_indices_signatures(
            ecash_parameters(),
            &[key_pair.secret_key()],
            &vec![key_pair.verification_key()],
            &key_pair.verification_key(),
            &[key_pair.index.unwrap()],
        )
        .unwrap();
        let pay_info = PayInfo {
            pay_info_bytes: [6u8; 72],
        };
        let pay_info2 = PayInfo {
            pay_info_bytes: [7u8; 72],
        };
        let spending = issued
            .prepare_for_spending(
                &key_pair.verification_key(),
                pay_info,
                &coin_indices_signatures,
            )
            .unwrap();
        let double_spending = issued2
            .prepare_for_spending(
                &key_pair.verification_key(),
                pay_info2,
                &coin_indices_signatures,
            )
            .unwrap();

        let storage1 = NymApiStorage::init(db_dir.path().join("storage.db"))
            .await
            .unwrap();
        let comm_channel = DummyCommunicationChannel::new(key_pair.verification_key().clone());
        let staged_key_pair = crate::ecash::KeyPair::new();
        staged_key_pair
            .set(KeyPairWithEpoch {
                keys: key_pair,
                issued_for_epoch: epoch,
            })
            .await;
        staged_key_pair.validate();
        let mut rng = OsRng;
        let identity = identity::KeyPair::new(&mut rng);

        let rocket = rocket::build().attach(crate::ecash::stage(
            nyxd_client.clone(),
            identity,
            staged_key_pair,
            comm_channel.clone(),
            storage1.clone(),
        ));

        let client = Client::tracked(rocket)
            .await
            .expect("valid rocket instance");

        let proposal_id = 42;
        // The address is not used, so we can use a duplicate
        let gateway_cosmos_addr = validator_address.clone();
        let req = VerifyEcashCredentialBody::new(
            spending.clone(),
            gateway_cosmos_addr.clone(),
            Some(proposal_id),
        );

        // Test endpoint with not proposal for the proposal id
        let response = client
            .post(format!(
                "/{}/{}/{}/{}",
                API_VERSION, COCONUT_ROUTES, BANDWIDTH, ECASH_VERIFY_ONLINE_CREDENTIAL
            ))
            .json(&req)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::BadRequest);
        assert_eq!(
            response.into_string().await.unwrap(),
            CoconutError::IncorrectProposal {
                reason: "proposal not found".to_string()
            }
            .to_string()
        );

        let mut proposal = Proposal {
            title: String::new(),
            description: String::from(
                "65TETnK13g1sSUVgrMHcwMUBmu2xUyEXQiCiREJxXpacFoR5GbniRHwqdo4VwWv7Sd",
            ),
            msgs: vec![],
            status: cw3::Status::Open,
            expires: cw_utils::Expiration::Never {},
            threshold: cw_utils::Threshold::AbsolutePercentage {
                percentage: Decimal::from_ratio(2u32, 3u32),
            },
            total_weight: chain.lock().unwrap().total_group_weight(),
            votes: Votes::yes(0),
            proposer: Addr::unchecked("proposer"),
            deposit: None,
            start_height: 0,
        };

        // Test the endpoint with a different blinded serial number in the description

        chain
            .lock()
            .unwrap()
            .multisig_contract
            .proposals
            .insert(proposal_id, proposal.clone());
        let response = client
            .post(format!(
                "/{}/{}/{}/{}",
                API_VERSION, COCONUT_ROUTES, BANDWIDTH, ECASH_VERIFY_ONLINE_CREDENTIAL
            ))
            .json(&req)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let verify_credential_response = serde_json::from_str::<VerifyEcashCredentialResponse>(
            &response.into_string().await.unwrap(),
        )
        .unwrap();

        assert_eq!(
            verify_credential_response,
            VerifyEcashCredentialResponse::InvalidFormat(
                "incorrect blinded serial number in description".to_string()
            )
        );

        // Test the endpoint with no msg in the proposal action
        proposal.description = spending.payment.serial_number_bs58();
        chain.lock().unwrap().reset_votes();
        chain
            .lock()
            .unwrap()
            .multisig_contract
            .proposals
            .insert(proposal_id, proposal.clone());
        let response = client
            .post(format!(
                "/{}/{}/{}/{}",
                API_VERSION, COCONUT_ROUTES, BANDWIDTH, ECASH_VERIFY_ONLINE_CREDENTIAL
            ))
            .json(&req)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let verify_credential_response = serde_json::from_str::<VerifyEcashCredentialResponse>(
            &response.into_string().await.unwrap(),
        )
        .unwrap();
        assert_eq!(
            verify_credential_response,
            VerifyEcashCredentialResponse::InvalidFormat(
                "action is not to spend_credential".to_string()
            )
        );

        // Test the endpoint with a credential that doesn't verify correctly
        let mut bad_spending = spending.clone();
        bad_spending.payment.kappa = bad_spending.payment.kappa + bad_spending.payment.kappa;
        let bad_req = VerifyEcashCredentialBody::new(
            bad_spending.clone(),
            gateway_cosmos_addr.clone(),
            Some(proposal_id),
        );
        chain.lock().unwrap().reset_votes();
        chain
            .lock()
            .unwrap()
            .multisig_contract
            .proposals
            .insert(proposal_id, proposal.clone());

        let response = client
            .post(format!(
                "/{}/{}/{}/{}",
                API_VERSION, COCONUT_ROUTES, BANDWIDTH, ECASH_VERIFY_ONLINE_CREDENTIAL
            ))
            .json(&bad_req)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let verify_credential_response = serde_json::from_str::<VerifyEcashCredentialResponse>(
            &response.into_string().await.unwrap(),
        )
        .unwrap();
        assert_eq!(
            verify_credential_response,
            VerifyEcashCredentialResponse::Refused
        );
        assert_eq!(
            cw3::Status::Rejected,
            chain
                .lock()
                .unwrap()
                .multisig_contract
                .proposals
                .get(&proposal_id)
                .unwrap()
                .status
        );

        // Test the endpoint with every dependency met
        let msg = nym_ecash_contract_common::msg::ExecuteMsg::SpendCredential {
            serial_number: spending.payment.serial_number_bs58(),
            gateway_cosmos_address: gateway_cosmos_addr.to_string(),
        };
        let cosmos_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: String::new(),
            msg: to_binary(&msg).unwrap(),
            funds: vec![],
        });
        proposal.msgs = vec![cosmos_msg];
        chain.lock().unwrap().reset_votes();
        chain
            .lock()
            .unwrap()
            .multisig_contract
            .proposals
            .insert(proposal_id, proposal.clone());
        let response = client
            .post(format!(
                "/{}/{}/{}/{}",
                API_VERSION, COCONUT_ROUTES, BANDWIDTH, ECASH_VERIFY_ONLINE_CREDENTIAL
            ))
            .json(&req)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let verify_credential_response = serde_json::from_str::<VerifyEcashCredentialResponse>(
            &response.into_string().await.unwrap(),
        )
        .unwrap();
        assert_eq!(
            verify_credential_response,
            VerifyEcashCredentialResponse::Accepted
        );
        assert_eq!(
            cw3::Status::Passed,
            chain
                .lock()
                .unwrap()
                .multisig_contract
                .proposals
                .get(&proposal_id)
                .unwrap()
                .status
        );

        // Test the endpoint with the credential already sent
        chain.lock().unwrap().reset_votes();
        let response = client
            .post(format!(
                "/{}/{}/{}/{}",
                API_VERSION, COCONUT_ROUTES, BANDWIDTH, ECASH_VERIFY_ONLINE_CREDENTIAL
            ))
            .json(&req)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let verify_credential_response = serde_json::from_str::<VerifyEcashCredentialResponse>(
            &response.into_string().await.unwrap(),
        )
        .unwrap();
        assert_eq!(
            verify_credential_response,
            VerifyEcashCredentialResponse::AlreadySent
        );

        // Test the endpoint with the credential already spent
        let double_spend_req = VerifyEcashCredentialBody::new(
            double_spending.clone(),
            gateway_cosmos_addr.clone(),
            Some(proposal_id),
        );
        chain.lock().unwrap().reset_votes();
        let response = client
            .post(format!(
                "/{}/{}/{}/{}",
                API_VERSION, COCONUT_ROUTES, BANDWIDTH, ECASH_VERIFY_ONLINE_CREDENTIAL
            ))
            .json(&double_spend_req)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let verify_credential_response = serde_json::from_str::<VerifyEcashCredentialResponse>(
            &response.into_string().await.unwrap(),
        )
        .unwrap();
        assert_eq!(
            verify_credential_response,
            VerifyEcashCredentialResponse::DoubleSpend
        );
    }
}

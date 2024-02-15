// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::coconut::error::{CoconutError, Result};
use crate::coconut::keys::KeyPairWithEpoch;
use crate::coconut::state::State;
use crate::coconut::storage::CoconutStorageExt;
use crate::support::storage::NymApiStorage;
use async_trait::async_trait;
use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{
    coin, from_binary, to_binary, Addr, Binary, BlockInfo, CosmosMsg, Decimal, MessageInfo, WasmMsg,
};
use cw3::{Proposal, ProposalResponse, Vote, VoteInfo, VoteResponse, Votes};
use cw4::{Cw4Contract, MemberResponse};
use nym_api_requests::coconut::models::{IssuedCredentialBody, IssuedCredentialResponse};
use nym_api_requests::coconut::{BlindSignRequestBody, BlindedSignatureResponse};
use nym_coconut::{BlindedSignature, Parameters};
use nym_coconut_bandwidth_contract_common::events::{
    DEPOSITED_FUNDS_EVENT_TYPE, DEPOSIT_ENCRYPTION_KEY, DEPOSIT_IDENTITY_KEY, DEPOSIT_INFO,
    DEPOSIT_VALUE,
};
use nym_coconut_bandwidth_contract_common::spend_credential::SpendCredentialResponse;
use nym_coconut_dkg_common::dealer::{DealerDetails, DealerDetailsResponse, DealerType};
use nym_coconut_dkg_common::dealing::{
    DealerDealingsStatusResponse, DealingChunkInfo, DealingMetadata, DealingStatus,
    DealingStatusResponse, PartialContractDealing,
};
use nym_coconut_dkg_common::event_attributes::{DKG_PROPOSAL_ID, NODE_INDEX};
use nym_coconut_dkg_common::types::{
    ChunkIndex, DealingIndex, EncodedBTEPublicKeyWithProof, Epoch, EpochId, EpochState,
    PartialContractDealingData, State as ContractState,
};
use nym_coconut_dkg_common::verification_key::{ContractVKShare, VerificationKeyShare};
use nym_coconut_interface::VerificationKey;
use nym_config::defaults::VOUCHER_INFO;
use nym_contracts_common::IdentityKey;
use nym_credentials::coconut::bandwidth::BandwidthVoucher;
use nym_crypto::asymmetric::{encryption, identity};
use nym_dkg::{NodeIndex, Threshold};
use nym_mixnet_contract_common::BlockHeight;
use nym_validator_client::nym_api::routes::{
    API_VERSION, BANDWIDTH, COCONUT_BLIND_SIGN, COCONUT_ROUTES,
};
use nym_validator_client::nyxd::cosmwasm_client::logs::Log;
use nym_validator_client::nyxd::cosmwasm_client::types::ExecuteResult;
use nym_validator_client::nyxd::Coin;
use nym_validator_client::nyxd::{
    AccountId, Algorithm, Event, EventAttribute, ExecTxResult, Fee, Hash, TxResponse,
};
use rand_07::rngs::OsRng;
use rand_07::RngCore;
use rocket::http::Status;
use rocket::local::asynchronous::Client;
use std::collections::{BTreeMap, HashMap};
use std::mem;
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

    // map of epoch id -> dealer -> dealings
    pub(crate) dealings: HashMap<EpochId, HashMap<String, HashMap<DealingIndex, Dealing>>>,

    // map of epoch id -> dealer -> vk share
    pub(crate) verification_shares: HashMap<EpochId, HashMap<String, ContractVKShare>>,

    pub(crate) epoch: Epoch,
    pub(crate) contract_state: ContractState,
    pub(crate) threshold: Option<Threshold>,
}

impl FakeDkgContractState {
    pub(crate) fn verified_dealers(&self) -> Vec<Addr> {
        let epoch_id = self.epoch.epoch_id;
        let Some(shares) = self.verification_shares.get(&epoch_id) else {
            return Vec::new();
        };

        shares
            .values()
            .filter(|s| s.verified)
            .map(|s| s.owner.clone())
            .collect()
    }

    fn reset_dkg_state(&mut self) {
        self.threshold = None;
        let dealers = mem::take(&mut self.dealers);
        for (index, details) in dealers {
            self.past_dealers.insert(index, details);
        }
    }

    pub(crate) fn reset_epoch_in_reshare_mode(&mut self, block_height: BlockHeight) {
        if let Some(initial_dealers) = self.initial_dealers.as_mut() {
            initial_dealers.initial_height = block_height;
        } else {
            self.initial_dealers = Some(InitialReplacementData {
                initial_dealers: self.verified_dealers(),
                initial_height: block_height,
            })
        }

        self.reset_dkg_state();
        self.epoch.state = EpochState::PublicKeySubmission { resharing: true };
        self.epoch.epoch_id += 1;
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
    pub(crate) spent_credentials: HashMap<String, SpendCredentialResponse>,
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

        FakeChainState {
            _counters: Default::default(),

            block_info: mock_env().block,
            txs: HashMap::new(),

            dkg_contract: FakeDkgContractState {
                address: dkg_contract.as_ref().parse().unwrap(),
                dealers: HashMap::new(),
                past_dealers: Default::default(),

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
                initial_dealers: None,
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
                spent_credentials: Default::default(),
            },
        }
    }
}

impl FakeChainState {
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
        self.dkg_contract
            .reset_epoch_in_reshare_mode(self.block_info.height)
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

    // pub fn with_tx_db(mut self, tx_db: &Arc<RwLock<HashMap<Hash, TxResponse>>>) -> Self {
    //     todo!()
    //     // self.tx_db = Arc::clone(tx_db);
    //     // self
    // }
    //
    // pub fn with_proposal_db(
    //     mut self,
    //     proposal_db: &Arc<RwLock<HashMap<u64, ProposalResponse>>>,
    // ) -> Self {
    //     todo!()
    //     // self.proposal_db = Arc::clone(proposal_db);
    //     // self
    // }
    //
    // pub fn with_spent_credential_db(
    //     mut self,
    //     spent_credential_db: &Arc<RwLock<HashMap<String, SpendCredentialResponse>>>,
    // ) -> Self {
    //     todo!()
    //     // self.spent_credential_db = Arc::clone(spent_credential_db);
    //     // self
    // }
    //
    // pub fn _with_epoch(mut self, epoch: &Arc<RwLock<Epoch>>) -> Self {
    //     todo!()
    //     // self.epoch = Arc::clone(epoch);
    //     // self
    // }
    //
    // pub fn with_dealer_details(
    //     mut self,
    //     dealer_details: &Arc<RwLock<HashMap<String, (DealerDetails, bool)>>>,
    // ) -> Self {
    //     todo!()
    //     // self.dealer_details = Arc::clone(dealer_details);
    //     // self
    // }
    //
    // pub fn with_threshold(mut self, threshold: &Arc<RwLock<Option<Threshold>>>) -> Self {
    //     todo!()
    //     // self.threshold = Arc::clone(threshold);
    //     // self
    // }
    //
    // // it's a really bad practice, but I'm not going to be changing it now...
    // #[allow(clippy::type_complexity)]
    // pub fn with_dealings(
    //     mut self,
    //     dealings: &Arc<RwLock<HashMap<EpochId, HashMap<String, Vec<PartialContractDealing>>>>>,
    // ) -> Self {
    //     todo!()
    //     // self.dealings = Arc::clone(dealings);
    //     // self
    // }
    //
    // pub fn with_verification_share(
    //     mut self,
    //     verification_share: &Arc<RwLock<HashMap<String, ContractVKShare>>>,
    // ) -> Self {
    //     todo!()
    //     // self.verification_share = Arc::clone(verification_share);
    //     // self
    // }
    //
    // pub fn _with_group_db(
    //     mut self,
    //     group_db: &Arc<RwLock<HashMap<String, MemberResponse>>>,
    // ) -> Self {
    //     todo!()
    //     // self.group_db = Arc::clone(group_db);
    //     // self
    // }
    //
    // pub fn with_initial_dealers_db(
    //     mut self,
    //     initial_dealers: &Arc<RwLock<Option<InitialReplacementData>>>,
    // ) -> Self {
    //     todo!()
    //     // self.initial_dealers_db = Arc::clone(initial_dealers);
    //     // self
    // }

    async fn get_dealer_by_address(&self, address: &str) -> Option<DealerDetails> {
        let guard = self.state.lock().unwrap();
        for dealer in guard.dkg_contract.dealers.values() {
            if dealer.address.as_str() == address {
                return Some(dealer.clone());
            }
        }
        None
    }

    async fn get_past_dealer_by_address(&self, address: &str) -> Option<DealerDetails> {
        let guard = self.state.lock().unwrap();
        for dealer in guard.dkg_contract.past_dealers.values() {
            if dealer.address.as_str() == address {
                return Some(dealer.clone());
            }
        }
        None
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

    async fn get_tx(&self, tx_hash: Hash) -> Result<TxResponse> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .txs
            .get(&tx_hash)
            .cloned()
            .unwrap())
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
    ) -> Result<SpendCredentialResponse> {
        self.state
            .lock()
            .unwrap()
            .bandwidth_contract
            .spent_credentials
            .get(&blinded_serial_number)
            .cloned()
            .ok_or(CoconutError::InvalidCredentialStatus {
                status: String::from("spent credential not found"),
            })
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

        if let Some(details) = self.get_dealer_by_address(address).await {
            return Ok(DealerDetailsResponse {
                details: Some(details),
                dealer_type: DealerType::Current,
            });
        }

        if let Some(details) = self.get_past_dealer_by_address(address).await {
            return Ok(DealerDetailsResponse {
                details: Some(details),
                dealer_type: DealerType::Past,
            });
        }

        Ok(DealerDetailsResponse {
            details: None,
            dealer_type: DealerType::Unknown,
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
        Ok(self
            .state
            .lock()
            .unwrap()
            .dkg_contract
            .dealers
            .values()
            .cloned()
            .collect())
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
            .get(&(voter.clone(), proposal_id))
            .is_some()
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
        let assigned_index = if let Some(already_registered) = self
            .get_dealer_by_address(self.validator_address.as_ref())
            .await
        {
            // current dealer
            already_registered.assigned_index
        } else if let Some(registered_in_the_past) = self
            .get_past_dealer_by_address(self.validator_address.as_ref())
            .await
        {
            // past dealer
            let index = registered_in_the_past.assigned_index;
            let mut guard = self.state.lock().unwrap();
            guard
                .dkg_contract
                .dealers
                .insert(index, registered_in_the_past);

            index
        } else {
            // new dealer
            let mut guard = self.state.lock().unwrap();
            let assigned_index = guard._counters.next_node_index();

            guard.dkg_contract.dealers.insert(
                assigned_index,
                DealerDetails {
                    address: Addr::unchecked(self.validator_address.to_string()),
                    bte_public_key_with_proof,
                    ed25519_identity: identity_key,
                    announce_address,
                    assigned_index,
                },
            );
            assigned_index
        };
        let mut guard = self.state.lock().unwrap();
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
    async fn submit_verification_key_share(
        &self,
        share: VerificationKeyShare,
        resharing: bool,
    ) -> Result<ExecuteResult> {
        let address = self.validator_address.to_string();

        let Some(dealer_details) = self.get_dealer_by_address(&address).await else {
            // Just throw some error, not really the correct one
            return Err(CoconutError::DepositEncrKeyNotFound);
        };

        let mut chain = self.state.lock().unwrap();
        let dkg_contract = chain.dkg_contract.address.clone();
        let epoch_id = chain.dkg_contract.epoch.epoch_id;

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
}

#[derive(Clone, Debug)]
pub struct DummyCommunicationChannel {
    current_epoch: Arc<AtomicU64>,
    aggregated_verification_key: VerificationKey,
}

impl DummyCommunicationChannel {
    pub fn new(aggregated_verification_key: VerificationKey) -> Self {
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

    async fn aggregated_verification_key(&self, _epoch_id: EpochId) -> Result<VerificationKey> {
        Ok(self.aggregated_verification_key.clone())
    }
}

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

pub fn deposit_tx_fixture(voucher: &BandwidthVoucher) -> TxResponse {
    TxResponse {
        hash: voucher.tx_hash(),
        height: Default::default(),
        index: 0,
        tx_result: ExecTxResult {
            code: Default::default(),
            data: Default::default(),
            log: "".to_string(),
            info: "".to_string(),
            gas_wanted: 0,
            gas_used: 0,
            events: vec![Event {
                kind: format!("wasm-{}", DEPOSITED_FUNDS_EVENT_TYPE),
                attributes: vec![
                    EventAttribute {
                        key: DEPOSIT_VALUE.to_string(),
                        value: voucher.get_voucher_value(),
                        index: false,
                    },
                    EventAttribute {
                        key: DEPOSIT_INFO.to_string(),
                        value: VOUCHER_INFO.to_string(),
                        index: false,
                    },
                    EventAttribute {
                        key: DEPOSIT_IDENTITY_KEY.to_string(),
                        value: voucher.identity_key().public_key().to_base58_string(),
                        index: false,
                    },
                    EventAttribute {
                        key: DEPOSIT_ENCRYPTION_KEY.parse().unwrap(),
                        value: voucher.encryption_key().public_key().to_base58_string(),
                        index: false,
                    },
                ],
            }],
            codespace: "".to_string(),
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

pub fn voucher_request_fixture<C: Into<Coin>>(
    amount: C,
    tx_hash: Option<String>,
) -> (BandwidthVoucher, BlindSignRequestBody) {
    let params = Parameters::new(4).unwrap();
    let mut rng = OsRng;
    let tx_hash = if let Some(provided) = &tx_hash {
        provided.parse().unwrap()
    } else {
        Hash::from_str("6B27412050B823E58BB38447D7870BBC8CBE3C51C905BEA89D459ACCDA80A00E").unwrap()
    };

    let identity_keypair = identity::KeyPair::new(&mut rng);
    let encryption_keypair = encryption::KeyPair::new(&mut rng);
    let id_priv =
        identity::PrivateKey::from_bytes(&identity_keypair.private_key().to_bytes()).unwrap();
    let enc_priv =
        encryption::PrivateKey::from_bytes(&encryption_keypair.private_key().to_bytes()).unwrap();

    let voucher = BandwidthVoucher::new(
        &params,
        amount.into().amount.to_string(),
        VOUCHER_INFO.to_string(),
        tx_hash,
        id_priv,
        enc_priv,
    );

    let request = BlindSignRequestBody::new(
        voucher.blind_sign_request().clone(),
        tx_hash,
        voucher.sign(),
        voucher.get_public_attributes_plain(),
    );

    (voucher, request)
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
        let mut rng = crate::coconut::tests::fixtures::test_rng_07([69u8; 32]);
        let params = Parameters::new(4).unwrap();
        let coconut_keypair = nym_coconut::ttp_keygen(&params, 1, 1).unwrap().remove(0);
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

        let staged_key_pair = crate::coconut::KeyPair::new();
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

        let rocket = rocket::build().attach(crate::coconut::stage(
            nyxd_client,
            TEST_COIN_DENOM.to_string(),
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

    fn add_tx(&self, hash: Hash, tx: TxResponse) {
        self.chain_state.lock().unwrap().txs.insert(hash, tx);
    }

    fn add_deposit_tx(&self, voucher: &BandwidthVoucher) {
        let mut guard = self.chain_state.lock().unwrap();
        let fixture = deposit_tx_fixture(voucher);
        guard.txs.insert(voucher.tx_hash(), fixture);
    }

    async fn issue_dummy_credential(&self) {
        let mut rng = OsRng;
        let mut tx_hash = [0u8; 32];
        rng.fill_bytes(&mut tx_hash);
        let tx_hash = Hash::from_bytes(Algorithm::Sha256, &tx_hash).unwrap();

        let (voucher, req) = voucher_request_fixture(coin(1234, "unym"), Some(tx_hash.to_string()));
        self.add_deposit_tx(&voucher);

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
    use crate::coconut::tests::helpers::init_chain;
    use nym_api_requests::coconut::{VerifyCredentialBody, VerifyCredentialResponse};
    use nym_coconut::tests::helpers::theta_from_keys_and_attributes;
    use nym_coconut::{hash_to_scalar, ttp_keygen};
    use nym_coconut_bandwidth_contract_common::spend_credential::SpendCredential;
    use nym_coconut_interface::Credential;
    use nym_validator_client::nym_api::routes::COCONUT_VERIFY_BANDWIDTH_CREDENTIAL;

    #[tokio::test]
    async fn already_issued() {
        let (_, request_body) = voucher_request_fixture(coin(1234, TEST_COIN_DENOM), None);
        let tx_hash = request_body.tx_hash;
        let tx_entry = tx_entry_fixture(tx_hash);

        let test_fixture = TestFixture::new().await;
        test_fixture.add_tx(tx_hash, tx_entry);

        let sig = blinded_signature_fixture();
        let commitments = request_body.encode_commitments();
        let public = request_body.public_attributes_plain.clone();
        test_fixture
            .storage
            .store_issued_credential(42, tx_hash, &sig, dummy_signature(), commitments, public)
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
        let params = Parameters::new(4).unwrap();
        let key_pair = ttp_keygen(&params, 1, 1).unwrap().remove(0);
        let tmp_dir = tempdir().unwrap();

        let storage = NymApiStorage::init(tmp_dir.path().join("storage.db"))
            .await
            .unwrap();
        let comm_channel = DummyCommunicationChannel::new(key_pair.verification_key().clone());
        let staged_key_pair = crate::coconut::KeyPair::new();
        staged_key_pair
            .set(KeyPairWithEpoch {
                keys: key_pair,
                issued_for_epoch: 1,
            })
            .await;
        staged_key_pair.validate();

        let state = State::new(
            nyxd_client,
            TEST_COIN_DENOM.to_string(),
            identity,
            staged_key_pair,
            comm_channel,
            storage.clone(),
        );

        let tx_hash = "6B27412050B823E58BB38447D7870BBC8CBE3C51C905BEA89D459ACCDA80A00E"
            .parse()
            .unwrap();
        assert!(state.already_issued(tx_hash).await.unwrap().is_none());

        let (_, request_body) = voucher_request_fixture(coin(1234, TEST_COIN_DENOM), None);
        let commitments = request_body.encode_commitments();
        let public = request_body.public_attributes_plain.clone();
        let sig = blinded_signature_fixture();
        storage
            .store_issued_credential(
                42,
                tx_hash,
                &sig,
                dummy_signature(),
                commitments.clone(),
                public.clone(),
            )
            .await
            .unwrap();

        assert_eq!(
            state
                .already_issued(tx_hash)
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
                tx_hash,
                &blinded_signature,
                dummy_signature(),
                commitments.clone(),
                public.clone(),
            )
            .await;
        assert!(storage_err.is_err());

        // And use a new hash to store a new signature
        let tx_hash = "97D64C38D6601B1F0FD3A82E20D252685CB7A210AFB0261018590659AB82B0BF"
            .parse()
            .unwrap();

        storage
            .store_issued_credential(
                42,
                tx_hash,
                &blinded_signature,
                dummy_signature(),
                commitments.clone(),
                public.clone(),
            )
            .await
            .unwrap();

        // Check that the same value for tx_hash is returned
        assert_eq!(
            state
                .already_issued(tx_hash)
                .await
                .unwrap()
                .unwrap()
                .to_bytes(),
            blinded_signature.to_bytes()
        );
    }

    #[tokio::test]
    async fn blind_sign_correct() {
        let tx_hash =
            Hash::from_str("7C41AF8266D91DE55E1C8F4712E6A952A165ED3D8C27C7B00428CBD0DE00A52B")
                .unwrap();

        let params = Parameters::new(4).unwrap();
        let mut rng = OsRng;
        let nym_api_identity = identity::KeyPair::new(&mut rng);

        let identity_keypair = identity::KeyPair::new(&mut rng);
        let encryption_keypair = encryption::KeyPair::new(&mut rng);
        let voucher = BandwidthVoucher::new(
            &params,
            "1234".to_string(),
            VOUCHER_INFO.to_string(),
            tx_hash,
            identity::PrivateKey::from_base58_string(
                identity_keypair.private_key().to_base58_string(),
            )
            .unwrap(),
            encryption::PrivateKey::from_bytes(&encryption_keypair.private_key().to_bytes())
                .unwrap(),
        );

        let key_pair = ttp_keygen(&params, 1, 1).unwrap().remove(0);
        let tmp_dir = tempdir().unwrap();
        let storage = NymApiStorage::init(tmp_dir.path().join("storage.db"))
            .await
            .unwrap();

        let chain = init_chain();

        let tx_entry = deposit_tx_fixture(&voucher);
        chain.lock().unwrap().txs.insert(tx_hash, tx_entry.clone());

        let nyxd_client = DummyClient::new(
            AccountId::from_str(TEST_REWARDING_VALIDATOR_ADDRESS).unwrap(),
            chain.clone(),
        );

        let comm_channel = DummyCommunicationChannel::new(key_pair.verification_key().clone());
        let staged_key_pair = crate::coconut::KeyPair::new();
        staged_key_pair
            .set(KeyPairWithEpoch {
                keys: key_pair,
                issued_for_epoch: 1,
            })
            .await;
        staged_key_pair.validate();

        let rocket = rocket::build().attach(crate::coconut::stage(
            nyxd_client,
            TEST_COIN_DENOM.to_string(),
            nym_api_identity,
            staged_key_pair,
            comm_channel,
            storage.clone(),
        ));
        let client = Client::tracked(rocket)
            .await
            .expect("valid rocket instance");

        let request_signature = voucher.sign();

        let request_body = BlindSignRequestBody::new(
            voucher.blind_sign_request().clone(),
            tx_hash,
            request_signature,
            voucher.get_public_attributes_plain(),
        );

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
        let params = Parameters::new(4).unwrap();
        let mut key_pairs = ttp_keygen(&params, 1, 1).unwrap();
        let voucher_value = 1234u64;
        let voucher_info = "voucher info";
        let public_attributes = [
            hash_to_scalar(voucher_value.to_string()),
            hash_to_scalar(voucher_info),
        ];
        let public_attributes_ref = vec![&public_attributes[0], &public_attributes[1]];
        let indices: Vec<u64> = key_pairs
            .iter()
            .enumerate()
            .map(|(idx, _)| (idx + 1) as u64)
            .collect();
        let theta =
            theta_from_keys_and_attributes(&params, &key_pairs, &indices, &public_attributes_ref)
                .unwrap();
        let key_pair = key_pairs.remove(0);
        let storage1 = NymApiStorage::init(db_dir.path().join("storage.db"))
            .await
            .unwrap();
        let comm_channel = DummyCommunicationChannel::new(key_pair.verification_key().clone());
        let staged_key_pair = crate::coconut::KeyPair::new();
        staged_key_pair
            .set(KeyPairWithEpoch {
                keys: key_pair,
                issued_for_epoch: 1,
            })
            .await;
        staged_key_pair.validate();
        let mut rng = OsRng;
        let identity = identity::KeyPair::new(&mut rng);

        let rocket = rocket::build().attach(crate::coconut::stage(
            nyxd_client.clone(),
            TEST_COIN_DENOM.to_string(),
            identity,
            staged_key_pair,
            comm_channel.clone(),
            storage1.clone(),
        ));

        let client = Client::tracked(rocket)
            .await
            .expect("valid rocket instance");

        let credential =
            Credential::new(4, theta.clone(), voucher_value, voucher_info.to_string(), 0);
        let proposal_id = 42;
        // The address is not used, so we can use a duplicate
        let gateway_cosmos_addr = validator_address.clone();
        let req =
            VerifyCredentialBody::new(credential.clone(), proposal_id, gateway_cosmos_addr.clone());

        // Test endpoint with not proposal for the proposal id
        let response = client
            .post(format!(
                "/{}/{}/{}/{}",
                API_VERSION, COCONUT_ROUTES, BANDWIDTH, COCONUT_VERIFY_BANDWIDTH_CREDENTIAL
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
            description: String::from("25mnnoCcUfeizfC85avvroFg2prpEZBgJbJM2SLtkgyyUkoAU3cqJiqWmg8cMHEPjfFf5sQF92SMAM2vbEoLZvUjenvXhadTLdA4TqMYArJpihyqirW2AhGoNehtcdcK5gnH"),
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
            start_height: 0
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
                API_VERSION, COCONUT_ROUTES, BANDWIDTH, COCONUT_VERIFY_BANDWIDTH_CREDENTIAL
            ))
            .json(&req)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::BadRequest);
        assert_eq!(
            response.into_string().await.unwrap(),
            CoconutError::IncorrectProposal {
                reason: "incorrect blinded serial number in description".to_string()
            }
            .to_string()
        );

        // Test the endpoint with no msg in the proposal action
        proposal.description = credential.blinded_serial_number();
        chain
            .lock()
            .unwrap()
            .multisig_contract
            .proposals
            .insert(proposal_id, proposal.clone());
        let response = client
            .post(format!(
                "/{}/{}/{}/{}",
                API_VERSION, COCONUT_ROUTES, BANDWIDTH, COCONUT_VERIFY_BANDWIDTH_CREDENTIAL
            ))
            .json(&req)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::BadRequest);
        assert_eq!(
            response.into_string().await.unwrap(),
            CoconutError::IncorrectProposal {
                reason: "action is not to release funds".to_string()
            }
            .to_string()
        );

        // Test the endpoint without any credential recorded in the Coconut Bandwidth Contract
        let funds = Coin::new(voucher_value as u128, TEST_COIN_DENOM);
        let msg = nym_coconut_bandwidth_contract_common::msg::ExecuteMsg::ReleaseFunds {
            funds: funds.clone().into(),
        };
        let cosmos_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: String::new(),
            msg: to_binary(&msg).unwrap(),
            funds: vec![],
        });
        proposal.msgs = vec![cosmos_msg];
        chain
            .lock()
            .unwrap()
            .multisig_contract
            .proposals
            .insert(proposal_id, proposal.clone());
        let response = client
            .post(format!(
                "/{}/{}/{}/{}",
                API_VERSION, COCONUT_ROUTES, BANDWIDTH, COCONUT_VERIFY_BANDWIDTH_CREDENTIAL
            ))
            .json(&req)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::BadRequest);
        assert_eq!(
            response.into_string().await.unwrap(),
            CoconutError::InvalidCredentialStatus {
                status: "spent credential not found".to_string()
            }
            .to_string()
        );

        chain
            .lock()
            .unwrap()
            .bandwidth_contract
            .spent_credentials
            .insert(
                credential.blinded_serial_number(),
                SpendCredentialResponse::new(None),
            );

        let response = client
            .post(format!(
                "/{}/{}/{}/{}",
                API_VERSION, COCONUT_ROUTES, BANDWIDTH, COCONUT_VERIFY_BANDWIDTH_CREDENTIAL
            ))
            .json(&req)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::BadRequest);
        assert_eq!(
            response.into_string().await.unwrap(),
            CoconutError::InvalidCredentialStatus {
                status: "Inexistent".to_string()
            }
            .to_string()
        );

        // Test the endpoint with a credential that doesn't verify correctly
        let mut spent_credential = SpendCredential::new(
            funds.clone().into(),
            credential.blinded_serial_number(),
            Addr::unchecked("unimportant"),
        );
        chain
            .lock()
            .unwrap()
            .bandwidth_contract
            .spent_credentials
            .insert(
                credential.blinded_serial_number(),
                SpendCredentialResponse::new(Some(spent_credential.clone())),
            );
        let bad_credential = Credential::new(
            4,
            theta.clone(),
            voucher_value,
            String::from("bad voucher info"),
            0,
        );
        let bad_req =
            VerifyCredentialBody::new(bad_credential, proposal_id, gateway_cosmos_addr.clone());
        let response = client
            .post(format!(
                "/{}/{}/{}/{}",
                API_VERSION, COCONUT_ROUTES, BANDWIDTH, COCONUT_VERIFY_BANDWIDTH_CREDENTIAL
            ))
            .json(&bad_req)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let verify_credential_response = serde_json::from_str::<VerifyCredentialResponse>(
            &response.into_string().await.unwrap(),
        )
        .unwrap();
        assert!(!verify_credential_response.verification_result);
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

        // Test the endpoint with a proposal that has a different value for the funds to be released
        // then what's in the credential
        let funds = Coin::new((voucher_value + 10) as u128, TEST_COIN_DENOM);
        let msg = nym_coconut_bandwidth_contract_common::msg::ExecuteMsg::ReleaseFunds {
            funds: funds.clone().into(),
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
                API_VERSION, COCONUT_ROUTES, BANDWIDTH, COCONUT_VERIFY_BANDWIDTH_CREDENTIAL
            ))
            .json(&req)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let verify_credential_response = serde_json::from_str::<VerifyCredentialResponse>(
            &response.into_string().await.unwrap(),
        )
        .unwrap();
        assert!(!verify_credential_response.verification_result);
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
        let funds = Coin::new(voucher_value as u128, TEST_COIN_DENOM);
        let msg = nym_coconut_bandwidth_contract_common::msg::ExecuteMsg::ReleaseFunds {
            funds: funds.clone().into(),
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
                API_VERSION, COCONUT_ROUTES, BANDWIDTH, COCONUT_VERIFY_BANDWIDTH_CREDENTIAL
            ))
            .json(&req)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let verify_credential_response = serde_json::from_str::<VerifyCredentialResponse>(
            &response.into_string().await.unwrap(),
        )
        .unwrap();
        assert!(verify_credential_response.verification_result);
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

        // Test the endpoint with the credential marked as Spent in the Coconut Bandwidth Contract
        spent_credential.mark_as_spent();
        chain
            .lock()
            .unwrap()
            .bandwidth_contract
            .spent_credentials
            .insert(
                credential.blinded_serial_number(),
                SpendCredentialResponse::new(Some(spent_credential)),
            );
        let response = client
            .post(format!(
                "/{}/{}/{}/{}",
                API_VERSION, COCONUT_ROUTES, BANDWIDTH, COCONUT_VERIFY_BANDWIDTH_CREDENTIAL
            ))
            .json(&req)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::BadRequest);
        assert_eq!(
            response.into_string().await.unwrap(),
            CoconutError::InvalidCredentialStatus {
                status: "Spent".to_string()
            }
            .to_string()
        );
    }
}

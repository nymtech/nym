// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! A [`crate::ecash::client::Client`] backed by the *real* coconut-dkg contract
//! (plus the real cw3 multisig and cw4 group) running under `cw_multi_test`.
//!
//! This is the counterpart to [`super::DummyClient`], which re-implements contract
//! behaviour by hand. Tests built on this module exercise the contract's own state
//! machine - deadlines, threshold computation and transition guards included - so
//! they can catch contract-level regressions that the hand-rolled chain cannot.
//!
//! `ContractTester` is not `Send` (`cw_multi_test::App` holds `Rc`-backed storage and
//! trait objects declared without `Send` bounds), while `DkgClient` requires
//! `Client + Send + Sync`. The tester therefore lives on its own thread and never
//! leaves it: callers submit closures over a channel and block on the result, so only
//! the closure and its owned return value ever cross the thread boundary, and the
//! compiler enforces that (`F: Send`, `T: Send`). A panic inside a job - a failed
//! contract assertion, say - is caught on the chain thread and re-raised at the
//! calling `with()`, payload intact.
//!
//! Only the DKG surface is served. The ecash-contract methods of the trait have no
//! contract behind them here and return an error rather than a plausible-looking
//! answer, so a test that strays onto that path fails loudly.

use crate::ecash::client::Client;
use crate::ecash::error::{EcashError, Result};
use async_trait::async_trait;
use cosmwasm_std::testing::message_info;
use cosmwasm_std::{Addr, Event as CosmwasmEvent};
use cw3::{ProposalListResponse, ProposalResponse, VoteResponse};
use cw4::MemberResponse;
use cw_multi_test::AppResponse;
use nym_coconut_dkg::testable_dkg_contract::{
    init_contract_tester_with_group_members, DkgContract, DkgContractTesterExt, GroupContract,
    MultisigContract,
};
use nym_coconut_dkg_common::dealer::{
    DealerDetails, DealerDetailsResponse, PagedDealerResponse, RegisteredDealerDetails,
};
use nym_coconut_dkg_common::dealing::{
    DealerDealingsStatusResponse, DealingChunkInfo, DealingChunkResponse, DealingMetadata,
    DealingMetadataResponse, DealingStatusResponse, PartialContractDealing,
};
use nym_coconut_dkg_common::msg::{ExecuteMsg as DkgExecuteMsg, QueryMsg as DkgQueryMsg};
use nym_coconut_dkg_common::types::{
    ChunkIndex, DealingIndex, EncodedBTEPublicKeyWithProof, Epoch, EpochId,
    PartialContractDealingData, State as ContractState, StateAdvanceResponse,
};
use nym_coconut_dkg_common::verification_key::{
    ContractVKShare, PagedVKSharesResponse, VerificationKeyShare, VkShareResponse,
};
use nym_contracts_common::IdentityKey;
use nym_contracts_common_testing::{AdminExt, ChainOpts, ContractOpts, ContractTester};
use nym_dkg::Threshold;
use nym_ecash_contract_common::blacklist::BlacklistedAccountResponse;
use nym_ecash_contract_common::deposit::{DepositId, DepositResponse};
use nym_validator_client::coconut::usable_hickory_ecash_api_clients;
use nym_validator_client::nyxd::cosmwasm_client::logs::Log;
use nym_validator_client::nyxd::cosmwasm_client::types::ExecuteResult;
use nym_validator_client::nyxd::{AccountId, Fee};
use nym_validator_client::EcashApiClient;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use tendermint::Hash;

/// A unit of work to run against the chain, on the thread that owns it.
type Job = Box<dyn FnOnce(&mut ContractChain) + Send>;

/// A cloneable handle to a contract chain running on its own dedicated thread.
#[derive(Clone)]
pub(crate) struct SharedContractChain {
    jobs: mpsc::Sender<Job>,
}

pub(crate) struct ContractChain {
    pub(crate) tester: ContractTester<DkgContract>,
    tx_counter: AtomicU64,
}

impl SharedContractChain {
    /// Stand up a fresh chain with `group_members` addresses already in the cw4 group.
    /// Dealers must be group members, so these are the addresses controllers may use.
    pub(crate) fn new(group_members: usize) -> Self {
        let (jobs, incoming) = mpsc::channel::<Job>();

        // the tester cannot change threads, so it is built on the thread that owns it
        // for its entire lifetime; the loop (and thread) ends once the last handle is
        // dropped and the channel closes
        std::thread::Builder::new()
            .name("dkg-contract-chain".to_string())
            .spawn(move || {
                let mut chain = ContractChain {
                    tester: init_contract_tester_with_group_members(group_members),
                    tx_counter: AtomicU64::new(0),
                };
                while let Ok(job) = incoming.recv() {
                    job(&mut chain);
                }
            })
            .expect("failed to spawn the contract chain thread");

        SharedContractChain { jobs }
    }

    /// Run `f` against the chain and block until it returns. If `f` panics, the panic
    /// is re-raised here with its original payload; the chain thread itself survives.
    pub(crate) fn with<T, F>(&self, f: F) -> T
    where
        F: FnOnce(&mut ContractChain) -> T + Send + 'static,
        T: Send + 'static,
    {
        let (result_tx, result_rx) = mpsc::sync_channel(1);
        self.jobs
            .send(Box::new(move |chain| {
                // AssertUnwindSafe: on a caught panic the chain may be left mid-operation,
                // but the panic is re-raised at the call site, so the test is failing anyway
                let result = std::panic::catch_unwind(AssertUnwindSafe(|| f(chain)));
                // a send failure means the caller has already given up on the result
                let _ = result_tx.send(result);
            }))
            .expect("the contract chain thread has terminated");

        match result_rx
            .recv()
            .expect("the contract chain thread dropped a job without responding")
        {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    /// The cw4 group members, in the order the tester created them.
    pub(crate) fn group_member_addresses(&self) -> Vec<AccountId> {
        self.with(|chain| {
            chain
                .tester
                .group_members()
                .iter()
                .map(unchecked_account_id)
                .collect()
        })
    }

    /// The admin of the DKG contract (and of the cw4 group).
    pub(crate) fn admin(&self) -> AccountId {
        self.with(|chain| unchecked_account_id(&chain.tester.admin_unchecked()))
    }

    pub(crate) fn epoch(&self) -> Epoch {
        self.with(|chain| chain.tester.epoch())
    }

    pub(crate) fn advance_time_by(&self, secs: u64) {
        self.with(move |chain| chain.tester.advance_time_by(secs))
    }

    pub(crate) fn add_group_member(&self, address: AccountId) {
        self.with(move |chain| {
            chain
                .tester
                .add_group_member(Addr::unchecked(address.as_ref()))
        })
    }

    /// Derive a fresh bech32 address the same way the tester derives its own.
    pub(crate) fn make_address(&self, label: String) -> AccountId {
        self.with(move |chain| unchecked_account_id(&chain.tester.addr_make(&label)))
    }

    pub(crate) fn key_size(&self) -> u32 {
        self.with(|chain| chain.tester.key_size())
    }

    /// Every proposal held by the real cw3 multisig, with its current status.
    pub(crate) fn proposals(&self) -> Vec<ProposalResponse> {
        self.with(|chain| {
            let mut proposals: Vec<ProposalResponse> = Vec::new();
            loop {
                let start_after = proposals.last().map(|proposal| proposal.id);
                let page: ProposalListResponse = chain
                    .query_contract::<MultisigContract, _, _>(
                        &nym_multisig_contract_common::msg::QueryMsg::ListProposals {
                            start_after,
                            limit: None,
                        },
                    )
                    .expect("failed to list multisig proposals");
                if page.proposals.is_empty() {
                    break;
                }
                proposals.extend(page.proposals);
            }
            proposals
        })
    }

    /// Drop the final byte of one dealing chunk, so the dealing no longer decodes.
    pub(crate) fn truncate_dealing_chunk(
        &self,
        epoch_id: EpochId,
        dealer: &AccountId,
        dealing_index: DealingIndex,
        chunk_index: ChunkIndex,
    ) {
        let dealer = Addr::unchecked(dealer.as_ref());
        self.with(move |chain| {
            chain
                .tester
                .truncate_dealing_chunk(epoch_id, &dealer, dealing_index, chunk_index)
        })
    }

    /// Drop the final byte of every chunk of every dealing this dealer submitted.
    pub(crate) fn truncate_all_dealings(&self, epoch_id: EpochId, dealer: &AccountId) {
        let dealer = Addr::unchecked(dealer.as_ref());
        self.with(move |chain| chain.tester.truncate_all_dealings(epoch_id, &dealer))
    }

    /// Alter a dealing's last byte without changing its length: it still decodes, but
    /// fails cryptographic verification.
    pub(crate) fn corrupt_dealing_payload(
        &self,
        epoch_id: EpochId,
        dealer: &AccountId,
        dealing_index: DealingIndex,
    ) {
        let dealer = Addr::unchecked(dealer.as_ref());
        self.with(move |chain| {
            chain
                .tester
                .corrupt_dealing_payload(epoch_id, &dealer, dealing_index)
        })
    }

    pub(crate) fn vk_share_value(
        &self,
        epoch_id: EpochId,
        owner: &AccountId,
    ) -> VerificationKeyShare {
        let owner = Addr::unchecked(owner.as_ref());
        self.with(move |chain| {
            chain
                .tester
                .vk_share(epoch_id, &owner)
                .expect("the dealer submitted no verification key share")
                .share
        })
    }

    /// Whether the contract considers this dealer's share verified.
    pub(crate) fn vk_share_verified(&self, epoch_id: EpochId, owner: &AccountId) -> bool {
        self.vk_share(epoch_id, owner).verified
    }

    pub(crate) fn vk_share(&self, epoch_id: EpochId, owner: &AccountId) -> ContractVKShare {
        let owner = Addr::unchecked(owner.as_ref());
        self.with(move |chain| {
            chain
                .tester
                .vk_share(epoch_id, &owner)
                .expect("the dealer submitted no verification key share")
        })
    }

    /// The cw3 multisig the DKG contract defers share verification to.
    pub(crate) fn multisig_address(&self) -> AccountId {
        self.with(|chain| unchecked_account_id(&chain.tester.multisig_contract()))
    }

    pub(crate) fn set_vk_share_value(
        &self,
        epoch_id: EpochId,
        owner: &AccountId,
        value: VerificationKeyShare,
    ) {
        let owner = Addr::unchecked(owner.as_ref());
        self.with(move |chain| {
            chain
                .tester
                .corrupt_vk_share(epoch_id, &owner, move |share| *share = value)
        })
    }

    pub(crate) fn epoch_threshold(&self, epoch_id: EpochId) -> Option<Threshold> {
        self.with(move |chain| {
            chain
                .query_dkg(&DkgQueryMsg::GetEpochThreshold { epoch_id })
                .expect("failed to query the epoch threshold")
        })
    }

    /// Execute a DKG message as `sender`, surfacing the contract's own error.
    pub(crate) fn execute_dkg(&self, sender: AccountId, msg: DkgExecuteMsg) -> Result<AppResponse> {
        self.with(move |chain| chain.execute_dkg(&sender, &msg))
    }

    /// Execute a passed multisig proposal as `sender`. The cw3 only checks that the
    /// sender is authorised, not that it proposed the thing, so any group member can
    /// execute any dealer's proposal - which is how a share can end up verified on
    /// chain without its own dealer having done anything.
    pub(crate) fn execute_multisig_proposal(
        &self,
        sender: AccountId,
        proposal_id: u64,
    ) -> Result<AppResponse> {
        self.with(move |chain| {
            let multisig = chain
                .tester
                .unchecked_contract_address::<MultisigContract>();
            chain
                .tester
                .execute_arbitrary_contract(
                    multisig,
                    message_info(&Addr::unchecked(sender.as_ref()), &[]),
                    &nym_multisig_contract_common::msg::ExecuteMsg::Execute { proposal_id },
                )
                .map_err(|err| contract_failure(format!("{err:#}")))
        })
    }
}

impl ContractChain {
    fn execute_dkg(&mut self, sender: &AccountId, msg: &DkgExecuteMsg) -> Result<AppResponse> {
        self.tester
            .execute_msg(Addr::unchecked(sender.as_ref()), msg)
            .map_err(|err| contract_failure(format!("{err:#}")))
    }

    fn query_dkg<T: DeserializeOwned>(&self, msg: &DkgQueryMsg) -> Result<T> {
        self.tester.query(msg).map_err(contract_failure)
    }

    fn query_contract<C, Q, T>(&self, msg: &Q) -> Result<T>
    where
        C: nym_contracts_common_testing::TestableNymContract,
        Q: Serialize + Debug,
        T: DeserializeOwned,
    {
        let address = self.tester.unchecked_contract_address::<C>();
        self.tester
            .query_arbitrary_contract(address, msg)
            .map_err(contract_failure)
    }

    fn execute_multisig<M: Serialize + Debug>(
        &mut self,
        sender: &AccountId,
        msg: &M,
    ) -> Result<AppResponse> {
        let multisig = self.tester.multisig_contract();
        let info = message_info(&Addr::unchecked(sender.as_ref()), &[]);
        self.tester
            .execute_arbitrary_contract(multisig, info, msg)
            .map_err(|err| contract_failure(format!("{err:#}")))
    }

    fn next_tx_hash(&self) -> Hash {
        use sha2::Digest;
        let cnt = self.tx_counter.fetch_add(1, Ordering::Relaxed);
        Hash::Sha256(sha2::Sha256::digest((cnt + 1).to_be_bytes()).into())
    }

    /// Repackage a `cw_multi_test` response as the `ExecuteResult` the DKG code expects.
    /// The attributes go into `logs` because the lookup helper prefers logs when present,
    /// and `logs` carries cosmwasm events directly - no abci conversion needed.
    fn to_execute_result(&self, response: AppResponse) -> ExecuteResult {
        let events = response
            .events
            .into_iter()
            .map(|event| {
                // cw_multi_test prefixes custom contract event types with "wasm-"; the
                // contract's own top-level attributes arrive under a plain "wasm" event,
                // which is also what the DKG code looks for
                let ty = event.ty.strip_prefix("wasm-").unwrap_or(&event.ty);
                let mut converted = CosmwasmEvent::new(ty);
                for attribute in event.attributes {
                    converted = converted.add_attribute(attribute.key, attribute.value);
                }
                converted
            })
            .collect();

        ExecuteResult {
            logs: vec![Log {
                msg_index: 0,
                events,
            }],
            msg_responses: Default::default(),
            events: Default::default(),
            transaction_hash: self.next_tx_hash(),
            gas_info: Default::default(),
        }
    }
}

fn contract_failure(err: impl std::fmt::Display) -> EcashError {
    EcashError::UnrecoverableState {
        reason: err.to_string(),
    }
}

fn unsupported(method: &str) -> EcashError {
    EcashError::UnrecoverableState {
        reason: format!(
            "'{method}' is not available on the contract-backed test chain: \
             it only runs the DKG contract, not the ecash contract"
        ),
    }
}

fn unchecked_account_id(addr: &Addr) -> AccountId {
    addr.as_str()
        .parse()
        .expect("test chain produced an address that is not a valid AccountId")
}

/// A signer's view of the contract-backed chain.
#[derive(Clone)]
pub(crate) struct ContractChainClient {
    address: AccountId,
    chain: SharedContractChain,
}

impl ContractChainClient {
    pub(crate) fn new(address: AccountId, chain: SharedContractChain) -> Self {
        ContractChainClient { address, chain }
    }
}

#[async_trait]
impl Client for ContractChainClient {
    async fn address(&self) -> Result<AccountId> {
        Ok(self.address.clone())
    }

    async fn dkg_contract_address(&self) -> Result<AccountId> {
        Ok(self.chain.with(|chain| {
            unchecked_account_id(&chain.tester.unchecked_contract_address::<DkgContract>())
        }))
    }

    async fn get_deposit(&self, _deposit_id: DepositId) -> Result<DepositResponse> {
        Err(unsupported("get_deposit"))
    }

    async fn get_proposal(&self, proposal_id: u64) -> Result<ProposalResponse> {
        self.chain.with(move |chain| {
            chain.query_contract::<MultisigContract, _, _>(
                &nym_multisig_contract_common::msg::QueryMsg::Proposal { proposal_id },
            )
        })
    }

    async fn list_proposals(&self) -> Result<Vec<ProposalResponse>> {
        self.chain.with(|chain| {
            let mut proposals: Vec<ProposalResponse> = Vec::new();
            loop {
                let start_after = proposals.last().map(|proposal| proposal.id);
                let page: ProposalListResponse = chain.query_contract::<MultisigContract, _, _>(
                    &nym_multisig_contract_common::msg::QueryMsg::ListProposals {
                        start_after,
                        limit: None,
                    },
                )?;
                if page.proposals.is_empty() {
                    break;
                }
                proposals.extend(page.proposals);
            }
            Ok(proposals)
        })
    }

    async fn get_vote(&self, proposal_id: u64, voter: String) -> Result<VoteResponse> {
        self.chain.with(move |chain| {
            chain.query_contract::<MultisigContract, _, _>(
                &nym_multisig_contract_common::msg::QueryMsg::Vote { proposal_id, voter },
            )
        })
    }

    async fn get_blacklisted_account(
        &self,
        _public_key: String,
    ) -> Result<BlacklistedAccountResponse> {
        Err(unsupported("get_blacklisted_account"))
    }

    async fn contract_state(&self) -> Result<ContractState> {
        self.chain
            .with(|chain| chain.query_dkg(&DkgQueryMsg::GetState {}))
    }

    async fn get_current_epoch(&self) -> Result<Epoch> {
        self.chain
            .with(|chain| chain.query_dkg(&DkgQueryMsg::GetCurrentEpochState {}))
    }

    async fn group_member(&self, addr: String) -> Result<MemberResponse> {
        self.chain.with(move |chain| {
            chain.query_contract::<GroupContract, _, _>(
                &nym_group_contract_common::msg::QueryMsg::Member {
                    addr,
                    at_height: None,
                },
            )
        })
    }

    async fn get_current_epoch_threshold(&self) -> Result<Option<Threshold>> {
        self.chain
            .with(|chain| chain.query_dkg(&DkgQueryMsg::GetCurrentEpochThreshold {}))
    }

    async fn get_epoch_threshold(&self, epoch_id: EpochId) -> Result<Option<Threshold>> {
        self.chain
            .with(move |chain| chain.query_dkg(&DkgQueryMsg::GetEpochThreshold { epoch_id }))
    }

    async fn get_self_registered_dealer_details(&self) -> Result<DealerDetailsResponse> {
        let dealer_address = self.address.to_string();
        self.chain
            .with(move |chain| chain.query_dkg(&DkgQueryMsg::GetDealerDetails { dealer_address }))
    }

    async fn get_registered_dealer_details(
        &self,
        epoch_id: EpochId,
        dealer: String,
    ) -> Result<RegisteredDealerDetails> {
        self.chain.with(move |chain| {
            chain.query_dkg(&DkgQueryMsg::GetRegisteredDealer {
                dealer_address: dealer,
                epoch_id: Some(epoch_id),
            })
        })
    }

    async fn get_dealer_dealings_status(
        &self,
        epoch_id: EpochId,
        dealer: String,
    ) -> Result<DealerDealingsStatusResponse> {
        self.chain.with(move |chain| {
            chain.query_dkg(&DkgQueryMsg::GetDealerDealingsStatus { epoch_id, dealer })
        })
    }

    async fn get_dealing_status(
        &self,
        epoch_id: EpochId,
        dealer: String,
        dealing_index: DealingIndex,
    ) -> Result<DealingStatusResponse> {
        self.chain.with(move |chain| {
            chain.query_dkg(&DkgQueryMsg::GetDealingStatus {
                epoch_id,
                dealer,
                dealing_index,
            })
        })
    }

    async fn get_current_dealers(&self) -> Result<Vec<DealerDetails>> {
        self.chain.with(|chain| {
            let mut dealers: Vec<DealerDetails> = Vec::new();
            let mut start_after = None;
            loop {
                let page: PagedDealerResponse =
                    chain.query_dkg(&DkgQueryMsg::GetCurrentDealers {
                        limit: None,
                        start_after: start_after.take(),
                    })?;
                let next = page.start_next_after;
                dealers.extend(page.dealers);
                match next {
                    Some(next) => start_after = Some(next.to_string()),
                    None => break,
                }
            }
            Ok(dealers)
        })
    }

    async fn get_dealing_metadata(
        &self,
        epoch_id: EpochId,
        dealer: String,
        dealing_index: DealingIndex,
    ) -> Result<Option<DealingMetadata>> {
        self.chain.with(move |chain| {
            let response: DealingMetadataResponse =
                chain.query_dkg(&DkgQueryMsg::GetDealingsMetadata {
                    epoch_id,
                    dealer,
                    dealing_index,
                })?;
            Ok(response.metadata)
        })
    }

    async fn get_dealing_chunk(
        &self,
        epoch_id: EpochId,
        dealer: &str,
        dealing_index: DealingIndex,
        chunk_index: ChunkIndex,
    ) -> Result<Option<PartialContractDealingData>> {
        let dealer = dealer.to_string();
        self.chain.with(move |chain| {
            let response: DealingChunkResponse =
                chain.query_dkg(&DkgQueryMsg::GetDealingChunk {
                    epoch_id,
                    dealer,
                    dealing_index,
                    chunk_index,
                })?;
            Ok(response.chunk)
        })
    }

    async fn get_verification_key_share(
        &self,
        epoch_id: EpochId,
        dealer: String,
    ) -> Result<Option<ContractVKShare>> {
        self.chain.with(move |chain| {
            let response: VkShareResponse = chain.query_dkg(&DkgQueryMsg::GetVerificationKey {
                epoch_id,
                owner: dealer,
            })?;
            Ok(response.share)
        })
    }

    async fn get_verification_key_shares(&self, epoch_id: EpochId) -> Result<Vec<ContractVKShare>> {
        self.chain.with(move |chain| {
            let mut shares: Vec<ContractVKShare> = Vec::new();
            let mut start_after = None;
            loop {
                let page: PagedVKSharesResponse =
                    chain.query_dkg(&DkgQueryMsg::GetVerificationKeys {
                        epoch_id,
                        limit: None,
                        start_after: start_after.take(),
                    })?;
                let next = page.start_next_after;
                shares.extend(page.shares);
                match next {
                    Some(next) => start_after = Some(next.to_string()),
                    None => break,
                }
            }
            Ok(shares)
        })
    }

    async fn get_registered_ecash_clients(&self, epoch_id: EpochId) -> Result<Vec<EcashApiClient>> {
        // deliberately the same shared helper the production client uses, so this double
        // cannot drift from the behaviour under test
        Ok(usable_hickory_ecash_api_clients(
            self.get_verification_key_shares(epoch_id).await?,
        ))
    }

    async fn vote_proposal(
        &self,
        proposal_id: u64,
        vote_yes: bool,
        _fee: Option<Fee>,
    ) -> Result<()> {
        let sender = self.address.clone();
        let vote = if vote_yes {
            cw3::Vote::Yes
        } else {
            cw3::Vote::No
        };
        self.chain.with(move |chain| {
            chain.execute_multisig(
                &sender,
                &nym_multisig_contract_common::msg::ExecuteMsg::Vote { proposal_id, vote },
            )
        })?;
        Ok(())
    }

    async fn execute_proposal(&self, proposal_id: u64) -> Result<()> {
        let sender = self.address.clone();
        self.chain.with(move |chain| {
            chain.execute_multisig(
                &sender,
                &nym_multisig_contract_common::msg::ExecuteMsg::Execute { proposal_id },
            )
        })?;
        Ok(())
    }

    async fn can_advance_epoch_state(&self) -> Result<bool> {
        let response: StateAdvanceResponse = self
            .chain
            .with(|chain| chain.query_dkg(&DkgQueryMsg::CanAdvanceState {}))?;
        Ok(response.can_advance())
    }

    async fn advance_epoch_state(&self) -> Result<()> {
        self.chain
            .execute_dkg(self.address.clone(), DkgExecuteMsg::AdvanceEpochState {})?;
        Ok(())
    }

    async fn register_dealer(
        &self,
        bte_key: EncodedBTEPublicKeyWithProof,
        identity_key: IdentityKey,
        announce_address: String,
        resharing: bool,
    ) -> Result<ExecuteResult> {
        let sender = self.address.clone();
        self.chain.with(move |chain| {
            let response = chain.execute_dkg(
                &sender,
                &DkgExecuteMsg::RegisterDealer {
                    bte_key_with_proof: bte_key,
                    identity_key,
                    announce_address,
                    resharing,
                },
            )?;
            Ok(chain.to_execute_result(response))
        })
    }

    async fn submit_dealing_metadata(
        &self,
        dealing_index: DealingIndex,
        chunks: Vec<DealingChunkInfo>,
        resharing: bool,
    ) -> Result<ExecuteResult> {
        let sender = self.address.clone();
        self.chain.with(move |chain| {
            let response = chain.execute_dkg(
                &sender,
                &DkgExecuteMsg::CommitDealingsMetadata {
                    dealing_index,
                    chunks,
                    resharing,
                },
            )?;
            Ok(chain.to_execute_result(response))
        })
    }

    async fn submit_dealing_chunk(&self, chunk: PartialContractDealing) -> Result<ExecuteResult> {
        let sender = self.address.clone();
        self.chain.with(move |chain| {
            let response =
                chain.execute_dkg(&sender, &DkgExecuteMsg::CommitDealingsChunk { chunk })?;
            Ok(chain.to_execute_result(response))
        })
    }

    async fn submit_verification_key_share(
        &self,
        share: VerificationKeyShare,
        resharing: bool,
    ) -> Result<ExecuteResult> {
        let sender = self.address.clone();
        self.chain.with(move |chain| {
            let response = chain.execute_dkg(
                &sender,
                &DkgExecuteMsg::CommitVerificationKeyShare { share, resharing },
            )?;
            Ok(chain.to_execute_result(response))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_coconut_dkg_common::types::EpochState;

    #[test]
    fn panics_inside_jobs_propagate_to_the_caller() {
        let chain = SharedContractChain::new(1);

        let caught = std::panic::catch_unwind(AssertUnwindSafe(|| {
            chain.with(|_chain| panic!("boom from the chain thread"))
        }));
        let payload = caught.expect_err("the panic should have propagated");
        let message = payload
            .downcast_ref::<&str>()
            .expect("the panic payload should have been preserved");
        assert_eq!(*message, "boom from the chain thread");

        // and the chain survives for subsequent jobs
        assert_eq!(chain.epoch().state, EpochState::WaitingInitialisation);
    }

    #[tokio::test]
    async fn serves_the_ecash_client_trait_from_the_real_contract() -> anyhow::Result<()> {
        let chain = SharedContractChain::new(3);
        let members = chain.group_member_addresses();
        assert_eq!(members.len(), 3);

        let client = ContractChainClient::new(members[0].clone(), chain.clone());
        assert_eq!(
            client.get_current_epoch().await?.state,
            EpochState::WaitingInitialisation
        );

        // the real contract's guards fire: registration before initiation is rejected
        assert!(client
            .register_dealer(
                "bte-key".to_string(),
                "identity".to_string(),
                "http://localhost:8080".to_string(),
                false,
            )
            .await
            .is_err());

        chain.execute_dkg(chain.admin(), DkgExecuteMsg::InitiateDkg {})?;
        assert_eq!(
            client.get_current_epoch().await?.state,
            EpochState::PublicKeySubmission { resharing: false }
        );

        // registration now succeeds, and the node index survives the event repackaging
        let result = client
            .register_dealer(
                "bte-key".to_string(),
                "identity".to_string(),
                "http://localhost:8080".to_string(),
                false,
            )
            .await?;
        let node_index =
            nym_validator_client::nyxd::helpers::find_attribute_value_in_logs_or_events(
                &result.logs,
                &result.events,
                "wasm",
                nym_coconut_dkg_common::event_attributes::NODE_INDEX,
            );
        assert_eq!(node_index.as_deref(), Some("1"));

        // and a non-member is rejected by the real group check, not for some incidental reason
        let outsider = chain.make_address("outsider".to_string());
        let outsider_client = ContractChainClient::new(outsider, chain.clone());
        let refusal = outsider_client
            .register_dealer(
                "bte-key-2".to_string(),
                "identity-2".to_string(),
                "http://localhost:8081".to_string(),
                false,
            )
            .await
            .unwrap_err();
        assert!(
            format!("{refusal:?}").contains("not in the coconut signer group"),
            "rejected, but not by the group check: {refusal:?}"
        );

        Ok(())
    }
}

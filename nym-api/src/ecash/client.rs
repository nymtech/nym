// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::error::Result;
use cw3::{ProposalResponse, VoteResponse};
use cw4::MemberResponse;
use nym_coconut_dkg_common::dealer::{
    DealerDetails, DealerDetailsResponse, RegisteredDealerDetails,
};
use nym_coconut_dkg_common::dealing::{
    DealerDealingsStatusResponse, DealingChunkInfo, DealingMetadata, DealingStatusResponse,
    PartialContractDealing,
};
use nym_coconut_dkg_common::types::{
    ChunkIndex, DealingIndex, EncodedBTEPublicKeyWithProof, Epoch, EpochId,
    PartialContractDealingData, State,
};
use nym_coconut_dkg_common::verification_key::{ContractVKShare, VerificationKeyShare};
use nym_contracts_common::IdentityKey;
use nym_dkg::Threshold;
use nym_ecash_contract_common::blacklist::BlacklistedAccountResponse;
use nym_ecash_contract_common::deposit::{DepositId, DepositResponse};
use nym_ecash_contract_common::spend_credential::EcashSpentCredentialResponse;
use nym_validator_client::nyxd::cosmwasm_client::types::ExecuteResult;
use nym_validator_client::nyxd::{AccountId, Fee};

#[async_trait]
pub trait Client {
    async fn address(&self) -> AccountId;

    async fn dkg_contract_address(&self) -> Result<AccountId>;

    async fn get_deposit(&self, deposit_id: DepositId) -> Result<DepositResponse>;

    async fn get_proposal(&self, proposal_id: u64) -> Result<ProposalResponse>;

    async fn list_proposals(&self) -> Result<Vec<ProposalResponse>>;

    async fn get_vote(&self, proposal_id: u64, voter: String) -> Result<VoteResponse>;

    async fn get_spent_credential(
        &self,
        blinded_serial_number: String,
    ) -> Result<EcashSpentCredentialResponse>;

    async fn propose_for_blacklist(&self, public_key: String) -> Result<ExecuteResult>;
    async fn get_blacklisted_account(
        &self,
        public_key: String,
    ) -> Result<BlacklistedAccountResponse>;

    async fn contract_state(&self) -> Result<State>;

    async fn get_current_epoch(&self) -> Result<Epoch>;

    async fn group_member(&self, addr: String) -> Result<MemberResponse>;

    async fn get_current_epoch_threshold(&self) -> Result<Option<Threshold>>;

    async fn get_self_registered_dealer_details(&self) -> Result<DealerDetailsResponse>;

    async fn get_registered_dealer_details(
        &self,
        epoch_id: EpochId,
        dealer: String,
    ) -> Result<RegisteredDealerDetails>;

    async fn get_dealer_dealings_status(
        &self,
        epoch_id: EpochId,
        dealer: String,
    ) -> Result<DealerDealingsStatusResponse>;

    #[allow(dead_code)]
    async fn get_dealing_status(
        &self,
        epoch_id: EpochId,
        dealer: String,
        dealing_index: DealingIndex,
    ) -> Result<DealingStatusResponse>;

    async fn get_current_dealers(&self) -> Result<Vec<DealerDetails>>;

    #[allow(dead_code)]
    async fn get_dealing_metadata(
        &self,
        epoch_id: EpochId,
        dealer: String,
        dealing_index: DealingIndex,
    ) -> Result<Option<DealingMetadata>>;

    async fn get_dealing_chunk(
        &self,
        epoch_id: EpochId,
        dealer: &str,
        dealing_index: DealingIndex,
        chunk_index: ChunkIndex,
    ) -> Result<Option<PartialContractDealingData>>;

    async fn get_verification_key_share(
        &self,
        epoch_id: EpochId,
        dealer: String,
    ) -> Result<Option<ContractVKShare>>;

    async fn get_verification_key_shares(&self, epoch_id: EpochId) -> Result<Vec<ContractVKShare>>;

    async fn vote_proposal(&self, proposal_id: u64, vote_yes: bool, fee: Option<Fee>)
        -> Result<()>;

    async fn execute_proposal(&self, proposal_id: u64) -> Result<()>;

    async fn can_advance_epoch_state(&self) -> Result<bool>;

    async fn advance_epoch_state(&self) -> Result<()>;

    async fn register_dealer(
        &self,
        bte_key: EncodedBTEPublicKeyWithProof,
        identity_key: IdentityKey,
        announce_address: String,
        resharing: bool,
    ) -> Result<ExecuteResult>;

    async fn submit_dealing_metadata(
        &self,
        dealing_index: DealingIndex,
        chunks: Vec<DealingChunkInfo>,
        resharing: bool,
    ) -> Result<ExecuteResult>;

    async fn submit_dealing_chunk(&self, chunk: PartialContractDealing) -> Result<ExecuteResult>;

    async fn submit_verification_key_share(
        &self,
        share: VerificationKeyShare,
        resharing: bool,
    ) -> Result<ExecuteResult>;
}

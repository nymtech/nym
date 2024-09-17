// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::client::Client;
use crate::ecash::error::EcashError;
use cw3::{ProposalResponse, Status, VoteResponse};
use cw4::MemberResponse;
use nym_coconut_dkg_common::dealer::{DealerDetails, DealerDetailsResponse};
use nym_coconut_dkg_common::dealing::{
    DealerDealingsStatusResponse, DealingChunkInfo, PartialContractDealing,
};
use nym_coconut_dkg_common::types::{
    ChunkIndex, DealingIndex, EncodedBTEPublicKeyWithProof, Epoch, EpochId, NodeIndex,
    PartialContractDealingData, State as ContractState,
};
use nym_coconut_dkg_common::verification_key::{ContractVKShare, VerificationKeyShare};
use nym_contracts_common::IdentityKey;
use nym_dkg::Threshold;
use nym_validator_client::nyxd::cosmwasm_client::logs::NODE_INDEX;
use nym_validator_client::nyxd::cosmwasm_client::types::ExecuteResult;
use nym_validator_client::nyxd::helpers::find_attribute_value_in_logs_or_events;
use nym_validator_client::nyxd::AccountId;

pub(crate) struct DkgClient {
    inner: Box<dyn Client + Send + Sync>,
}

impl DkgClient {
    pub(crate) fn new<C>(nyxd_client: C) -> Self
    where
        C: Client + Send + Sync + 'static,
    {
        DkgClient {
            inner: Box::new(nyxd_client),
        }
    }

    pub(crate) async fn get_address(&self) -> AccountId {
        self.inner.address().await
    }

    pub(crate) async fn dkg_contract_address(&self) -> Result<AccountId, EcashError> {
        self.inner.dkg_contract_address().await
    }

    pub(crate) async fn get_current_epoch(&self) -> Result<Epoch, EcashError> {
        self.inner.get_current_epoch().await
    }

    pub(crate) async fn get_contract_state(&self) -> Result<ContractState, EcashError> {
        self.inner.contract_state().await
    }

    pub(crate) async fn group_member(&self) -> Result<MemberResponse, EcashError> {
        self.inner
            .group_member(self.get_address().await.to_string())
            .await
    }

    pub(crate) async fn get_current_epoch_threshold(
        &self,
    ) -> Result<Option<Threshold>, EcashError> {
        self.inner.get_current_epoch_threshold().await
    }

    pub(crate) async fn get_self_registered_dealer_details(
        &self,
    ) -> Result<DealerDetailsResponse, EcashError> {
        self.inner.get_self_registered_dealer_details().await
    }

    pub(crate) async fn dealer_in_epoch(
        &self,
        epoch_id: EpochId,
        dealer: String,
    ) -> Result<bool, EcashError> {
        self.inner
            .get_registered_dealer_details(epoch_id, dealer)
            .await
            .map(|d| d.details.is_some())
    }

    pub(crate) async fn get_current_dealers(&self) -> Result<Vec<DealerDetails>, EcashError> {
        self.inner.get_current_dealers().await
    }

    pub(crate) async fn get_dealings_statuses(
        &self,
        epoch_id: EpochId,
        dealer: String,
    ) -> Result<DealerDealingsStatusResponse, EcashError> {
        self.inner
            .get_dealer_dealings_status(epoch_id, dealer)
            .await
    }

    pub(crate) async fn get_dealing_chunk(
        &self,
        epoch_id: EpochId,
        dealer: &str,
        dealing_index: DealingIndex,
        chunk_index: ChunkIndex,
    ) -> Result<PartialContractDealingData, EcashError> {
        self.inner
            .get_dealing_chunk(epoch_id, dealer, dealing_index, chunk_index)
            .await?
            .ok_or(EcashError::MissingDealingChunk {
                epoch_id,
                dealer: dealer.to_string(),
                dealing_index,
                chunk_index,
            })
    }

    pub(crate) async fn get_verification_key_share<S: Into<String>>(
        &self,
        epoch_id: EpochId,
        address: S,
    ) -> Result<Option<ContractVKShare>, EcashError> {
        self.inner
            .get_verification_key_share(epoch_id, address.into())
            .await
    }

    pub(crate) async fn get_verification_own_key_share(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<ContractVKShare>, EcashError> {
        let address = self.inner.address().await;
        self.get_verification_key_share(epoch_id, address).await
    }

    pub(crate) async fn get_verification_key_shares(
        &self,
        epoch_id: EpochId,
    ) -> Result<Vec<ContractVKShare>, EcashError> {
        self.inner.get_verification_key_shares(epoch_id).await
    }

    pub(crate) async fn get_vote(&self, proposal_id: u64) -> Result<VoteResponse, EcashError> {
        let address = self.get_address().await.to_string();
        self.inner.get_vote(proposal_id, address).await
    }

    pub(crate) async fn list_proposals(&self) -> Result<Vec<ProposalResponse>, EcashError> {
        self.inner.list_proposals().await
    }

    pub(crate) async fn get_proposal_status(&self, proposal_id: u64) -> Result<Status, EcashError> {
        self.inner.get_proposal(proposal_id).await.map(|p| p.status)
    }

    pub(crate) async fn advance_epoch_state(&self) -> Result<(), EcashError> {
        self.inner.advance_epoch_state().await
    }

    pub(crate) async fn can_advance_epoch_state(&self) -> Result<bool, EcashError> {
        self.inner.can_advance_epoch_state().await
    }

    pub(crate) async fn register_dealer(
        &self,
        bte_key: EncodedBTEPublicKeyWithProof,
        identity_key: IdentityKey,
        announce_address: String,
        resharing: bool,
    ) -> Result<NodeIndex, EcashError> {
        let res = self
            .inner
            .register_dealer(bte_key, identity_key, announce_address, resharing)
            .await?;
        let node_index =
            find_attribute_value_in_logs_or_events(&res.logs, &res.events, "wasm", NODE_INDEX)
                .ok_or(EcashError::NodeIndexRecoveryError {
                    reason: String::from("node index not found"),
                })?
                .parse::<NodeIndex>()
                .map_err(|_| EcashError::NodeIndexRecoveryError {
                    reason: String::from("node index could not be parsed"),
                })?;

        Ok(node_index)
    }

    pub(crate) async fn submit_dealing_metadata(
        &self,
        dealing_index: DealingIndex,
        chunks: Vec<DealingChunkInfo>,
        resharing: bool,
    ) -> Result<(), EcashError> {
        self.inner
            .submit_dealing_metadata(dealing_index, chunks, resharing)
            .await?;
        Ok(())
    }

    pub(crate) async fn submit_dealing_chunk(
        &self,
        chunk: PartialContractDealing,
    ) -> Result<(), EcashError> {
        self.inner.submit_dealing_chunk(chunk).await?;
        Ok(())
    }

    pub(crate) async fn submit_verification_key_share(
        &self,
        share: VerificationKeyShare,
        resharing: bool,
    ) -> Result<ExecuteResult, EcashError> {
        self.inner
            .submit_verification_key_share(share.clone(), resharing)
            .await
    }

    pub(crate) async fn vote_verification_key_share(
        &self,
        proposal_id: u64,
        vote_yes: bool,
    ) -> Result<(), EcashError> {
        self.inner.vote_proposal(proposal_id, vote_yes, None).await
    }

    pub(crate) async fn execute_verification_key_share(
        &self,
        proposal_id: u64,
    ) -> Result<(), EcashError> {
        self.inner.execute_proposal(proposal_id).await
    }
}

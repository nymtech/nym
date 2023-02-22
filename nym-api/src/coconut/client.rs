// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::error::Result;
use coconut_bandwidth_contract_common::spend_credential::SpendCredentialResponse;
use coconut_dkg_common::dealer::{ContractDealing, DealerDetails, DealerDetailsResponse};
use coconut_dkg_common::types::{EncodedBTEPublicKeyWithProof, Epoch};
use coconut_dkg_common::verification_key::{ContractVKShare, VerificationKeyShare};
use contracts_common::dealings::ContractSafeBytes;
use cw3::ProposalResponse;
use dkg::Threshold;
use validator_client::nyxd::cosmwasm_client::types::ExecuteResult;
use validator_client::nyxd::{AccountId, Fee, TxResponse};

#[async_trait]
pub trait Client {
    async fn address(&self) -> AccountId;
    async fn get_tx(&self, tx_hash: &str) -> Result<TxResponse>;
    async fn get_proposal(&self, proposal_id: u64) -> Result<ProposalResponse>;
    async fn list_proposals(&self) -> Result<Vec<ProposalResponse>>;
    async fn get_spent_credential(
        &self,
        blinded_serial_number: String,
    ) -> Result<SpendCredentialResponse>;
    async fn get_current_epoch(&self) -> Result<Epoch>;
    async fn get_current_epoch_threshold(&self) -> Result<Option<Threshold>>;
    async fn get_self_registered_dealer_details(&self) -> Result<DealerDetailsResponse>;
    async fn get_current_dealers(&self) -> Result<Vec<DealerDetails>>;
    async fn get_dealings(&self, idx: usize) -> Result<Vec<ContractDealing>>;
    async fn get_verification_key_shares(&self) -> Result<Vec<ContractVKShare>>;
    async fn vote_proposal(&self, proposal_id: u64, vote_yes: bool, fee: Option<Fee>)
        -> Result<()>;
    async fn execute_proposal(&self, proposal_id: u64) -> Result<()>;
    async fn advance_epoch_state(&self) -> Result<()>;
    async fn register_dealer(
        &self,
        bte_key: EncodedBTEPublicKeyWithProof,
        announce_address: String,
    ) -> Result<ExecuteResult>;
    async fn submit_dealing(&self, dealing_bytes: ContractSafeBytes) -> Result<ExecuteResult>;
    async fn submit_verification_key_share(
        &self,
        share: VerificationKeyShare,
    ) -> Result<ExecuteResult>;
}

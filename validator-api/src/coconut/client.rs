// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::error::Result;
use coconut_bandwidth_contract_common::spend_credential::SpendCredentialResponse;
use coconut_dkg_common::dealer::DealerDetailsResponse;
use coconut_dkg_common::types::{EncodedBTEPublicKeyWithProof, EpochState};
use contracts_common::commitment::ContractSafeCommitment;
use multisig_contract_common::msg::ProposalResponse;
use validator_client::nymd::cosmwasm_client::types::ExecuteResult;
use validator_client::nymd::{AccountId, Fee, TxResponse};

#[async_trait]
pub trait Client {
    async fn address(&self) -> AccountId;
    async fn get_tx(&self, tx_hash: &str) -> Result<TxResponse>;
    async fn get_proposal(&self, proposal_id: u64) -> Result<ProposalResponse>;
    async fn get_spent_credential(
        &self,
        blinded_serial_number: String,
    ) -> Result<SpendCredentialResponse>;
    async fn get_current_epoch_state(&self) -> Result<EpochState>;
    async fn get_self_registered_dealer_details(&self) -> Result<DealerDetailsResponse>;
    async fn vote_proposal(&self, proposal_id: u64, vote_yes: bool, fee: Option<Fee>)
        -> Result<()>;
    async fn register_dealer(&self, bte_key: EncodedBTEPublicKeyWithProof)
        -> Result<ExecuteResult>;
    async fn submit_dealing_commitment(
        &self,
        commitment: ContractSafeCommitment,
    ) -> Result<ExecuteResult>;
}

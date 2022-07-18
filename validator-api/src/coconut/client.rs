// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::error::Result;
use coconut_bandwidth_contract_common::spend_credential::SpendCredentialResponse;
use multisig_contract_common::msg::ProposalResponse;
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
    async fn vote_proposal(&self, proposal_id: u64, vote_yes: bool, fee: Option<Fee>)
        -> Result<()>;
}

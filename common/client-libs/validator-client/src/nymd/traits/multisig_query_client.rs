// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd::error::NymdError;
use crate::nymd::{CosmWasmClient, NymdClient};

use multisig_contract_common::msg::{ProposalResponse, QueryMsg};

use async_trait::async_trait;

#[async_trait]
pub trait MultisigQueryClient {
    async fn get_proposal(&self, proposal_id: u64) -> Result<ProposalResponse, NymdError>;
}

#[async_trait]
impl<C: CosmWasmClient + Sync + Send> MultisigQueryClient for NymdClient<C> {
    async fn get_proposal(&self, proposal_id: u64) -> Result<ProposalResponse, NymdError> {
        let request = QueryMsg::Proposal { proposal_id };
        self.client
            .query_contract_smart(self.multisig_contract_address(), &request)
            .await
    }
}

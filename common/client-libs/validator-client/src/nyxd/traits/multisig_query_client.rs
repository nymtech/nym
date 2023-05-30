// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::error::NyxdError;
use crate::nyxd::{CosmWasmClient, NyxdClient};

use cw3::{ProposalListResponse, ProposalResponse};
use nym_multisig_contract_common::msg::QueryMsg;

use async_trait::async_trait;

#[async_trait]
pub trait MultisigQueryClient {
    async fn get_proposal(&self, proposal_id: u64) -> Result<ProposalResponse, NyxdError>;

    async fn list_proposals(
        &self,
        start_after: Option<u64>,
        limit: Option<u32>,
    ) -> Result<ProposalListResponse, NyxdError>;

    async fn get_all_proposals(&self) -> Result<Vec<ProposalResponse>, NyxdError> {
        let mut proposals = Vec::new();
        let mut start_after = None;

        loop {
            let mut paged_response = self.list_proposals(start_after.take(), None).await?;

            let last_id = paged_response.proposals.last().map(|prop| prop.id);
            proposals.append(&mut paged_response.proposals);

            if let Some(start_after_res) = last_id {
                start_after = Some(start_after_res)
            } else {
                break;
            }
        }

        Ok(proposals)
    }
}

#[async_trait]
impl<C: CosmWasmClient + Sync + Send> MultisigQueryClient for NyxdClient<C> {
    async fn get_proposal(&self, proposal_id: u64) -> Result<ProposalResponse, NyxdError> {
        let request = QueryMsg::Proposal { proposal_id };
        self.client
            .query_contract_smart(self.multisig_contract_address(), &request)
            .await
    }

    async fn list_proposals(
        &self,
        start_after: Option<u64>,
        limit: Option<u32>,
    ) -> Result<ProposalListResponse, NyxdError> {
        let request = QueryMsg::ListProposals { start_after, limit };
        self.client
            .query_contract_smart(self.multisig_contract_address(), &request)
            .await
    }
}

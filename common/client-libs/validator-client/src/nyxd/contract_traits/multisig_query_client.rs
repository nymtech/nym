// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::error::NyxdError;
use crate::nyxd::CosmWasmClient;
use async_trait::async_trait;
use cw3::{
    ProposalListResponse, ProposalResponse, VoteListResponse, VoteResponse, VoterDetail,
    VoterListResponse, VoterResponse,
};
use cw_utils::ThresholdResponse;
use nym_multisig_contract_common::msg::QueryMsg as MultisigQueryMsg;
use serde::Deserialize;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait MultisigQueryClient {
    async fn query_multisig_contract<T>(&self, query: MultisigQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>;

    async fn query_threshold(&self) -> Result<ThresholdResponse, NyxdError> {
        self.query_multisig_contract(MultisigQueryMsg::Threshold {})
            .await
    }

    async fn query_proposal(&self, proposal_id: u64) -> Result<ProposalResponse, NyxdError> {
        self.query_multisig_contract(MultisigQueryMsg::Proposal { proposal_id })
            .await
    }

    async fn list_proposals(
        &self,
        start_after: Option<u64>,
        limit: Option<u32>,
    ) -> Result<ProposalListResponse, NyxdError> {
        self.query_multisig_contract(MultisigQueryMsg::ListProposals { start_after, limit })
            .await
    }

    async fn reverse_proposals(
        &self,
        start_before: Option<u64>,
        limit: Option<u32>,
    ) -> Result<ProposalListResponse, NyxdError> {
        self.query_multisig_contract(MultisigQueryMsg::ReverseProposals {
            start_before,
            limit,
        })
        .await
    }

    async fn query_vote(&self, proposal_id: u64, voter: String) -> Result<VoteResponse, NyxdError> {
        self.query_multisig_contract(MultisigQueryMsg::Vote { proposal_id, voter })
            .await
    }

    async fn list_votes(
        &self,
        proposal_id: u64,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Result<VoteListResponse, NyxdError> {
        self.query_multisig_contract(MultisigQueryMsg::ListVotes {
            proposal_id,
            start_after,
            limit,
        })
        .await
    }

    async fn query_voter(&self, address: String) -> Result<VoterResponse, NyxdError> {
        self.query_multisig_contract(MultisigQueryMsg::Voter { address })
            .await
    }

    async fn list_voters(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Result<VoterListResponse, NyxdError> {
        self.query_multisig_contract(MultisigQueryMsg::ListVoters { start_after, limit })
            .await
    }

    async fn query_config(&self) -> Result<(), NyxdError> {
        unimplemented!("requires exporting state::Config type")
    }
}

// extension trait to the query client to deal with the paged queries
// (it didn't feel appropriate to combine it with the existing trait
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait PagedMultisigQueryClient: MultisigQueryClient {
    // can't use the macro due to different paging behaviour
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

    async fn get_all_voters(&self) -> Result<Vec<VoterDetail>, NyxdError> {
        let mut voters = Vec::new();
        let mut start_after = None;

        loop {
            let mut paged_response = self.list_voters(start_after.take(), None).await?;

            let last_voter = paged_response.voters.last().map(|prop| prop.addr.clone());
            voters.append(&mut paged_response.voters);

            if let Some(start_after_res) = last_voter {
                start_after = Some(start_after_res)
            } else {
                break;
            }
        }

        Ok(voters)
    }
}

#[async_trait]
impl<T> PagedMultisigQueryClient for T where T: MultisigQueryClient {}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> MultisigQueryClient for C
where
    C: CosmWasmClient + NymContractsProvider + Send + Sync,
{
    async fn query_multisig_contract<T>(&self, query: MultisigQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        let multisig_contract_address = &self
            .multisig_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("multisig contract"))?;
        self.query_contract_smart(multisig_contract_address, &query)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nyxd::contract_traits::tests::IgnoreValue;

    // it's enough that this compiles and clippy is happy about it
    #[allow(dead_code)]
    fn all_query_variants_are_covered<C: MultisigQueryClient + Send + Sync>(
        client: C,
        msg: MultisigQueryMsg,
    ) {
        match msg {
            MultisigQueryMsg::Threshold {} => client.query_threshold().ignore(),
            MultisigQueryMsg::Proposal { proposal_id } => {
                client.query_proposal(proposal_id).ignore()
            }
            MultisigQueryMsg::ListProposals { start_after, limit } => {
                client.list_proposals(start_after, limit).ignore()
            }
            MultisigQueryMsg::ReverseProposals {
                start_before,
                limit,
            } => client.reverse_proposals(start_before, limit).ignore(),
            MultisigQueryMsg::Vote { proposal_id, voter } => {
                client.query_vote(proposal_id, voter).ignore()
            }
            MultisigQueryMsg::ListVotes {
                proposal_id,
                start_after,
                limit,
            } => client.list_votes(proposal_id, start_after, limit).ignore(),
            MultisigQueryMsg::Voter { address } => client.query_voter(address).ignore(),
            MultisigQueryMsg::ListVoters { start_after, limit } => {
                client.list_voters(start_after, limit).ignore()
            }
            MultisigQueryMsg::Config {} => client.query_config().ignore(),
        };
    }
}

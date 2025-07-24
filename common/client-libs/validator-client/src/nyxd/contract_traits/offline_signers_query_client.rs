// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::collect_paged;
use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::error::NyxdError;
use crate::nyxd::CosmWasmClient;
use async_trait::async_trait;
use cosmrs::AccountId;
use cw_controllers::AdminResponse;
use nym_offline_signers_contract_common::msg::QueryMsg as OfflineSignersQueryMsg;
use serde::Deserialize;

pub use nym_offline_signers_contract_common::{
    ActiveProposalResponse, ActiveProposalsPagedResponse, Config, LastStatusResetDetails,
    LastStatusResetPagedResponse, LastStatusResetResponse, OfflineSignerDetails,
    OfflineSignerInformation, OfflineSignerResponse, OfflineSignersAddressesResponse,
    OfflineSignersPagedResponse, Proposal, ProposalId, ProposalResponse, ProposalWithResolution,
    ProposalsPagedResponse, SigningStatusAtHeightResponse, SigningStatusResponse, VoteDetails,
    VoteInformation, VoteResponse, VotesPagedResponse,
};

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait OfflineSignersQueryClient {
    async fn query_offline_signers_contract<T>(
        &self,
        query: OfflineSignersQueryMsg,
    ) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>;

    async fn admin(&self) -> Result<AdminResponse, NyxdError> {
        self.query_offline_signers_contract(OfflineSignersQueryMsg::Admin {})
            .await
    }

    async fn get_config(&self) -> Result<Config, NyxdError> {
        self.query_offline_signers_contract(OfflineSignersQueryMsg::GetConfig {})
            .await
    }

    async fn get_active_proposal(
        &self,
        signer: AccountId,
    ) -> Result<ActiveProposalResponse, NyxdError> {
        self.query_offline_signers_contract(OfflineSignersQueryMsg::GetActiveProposal {
            signer: signer.to_string(),
        })
        .await
    }

    async fn get_proposal(&self, proposal_id: ProposalId) -> Result<ProposalResponse, NyxdError> {
        self.query_offline_signers_contract(OfflineSignersQueryMsg::GetProposal { proposal_id })
            .await
    }

    async fn get_vote_information(
        &self,
        voter: AccountId,
        proposal: ProposalId,
    ) -> Result<VoteResponse, NyxdError> {
        self.query_offline_signers_contract(OfflineSignersQueryMsg::GetVoteInformation {
            voter: voter.to_string(),
            proposal,
        })
        .await
    }

    async fn get_offline_signer_information(
        &self,
        signer: AccountId,
    ) -> Result<OfflineSignerResponse, NyxdError> {
        self.query_offline_signers_contract(OfflineSignersQueryMsg::GetOfflineSignerInformation {
            signer: signer.to_string(),
        })
        .await
    }

    async fn get_offline_signers_addresses_at_height(
        &self,
        height: Option<u64>,
    ) -> Result<OfflineSignersAddressesResponse, NyxdError> {
        self.query_offline_signers_contract(
            OfflineSignersQueryMsg::GetOfflineSignersAddressesAtHeight { height },
        )
        .await
    }

    async fn get_last_status_reset(
        &self,
        signer: AccountId,
    ) -> Result<LastStatusResetResponse, NyxdError> {
        self.query_offline_signers_contract(OfflineSignersQueryMsg::GetLastStatusReset {
            signer: signer.to_string(),
        })
        .await
    }

    async fn get_active_proposals_paged(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Result<ActiveProposalsPagedResponse, NyxdError> {
        self.query_offline_signers_contract(OfflineSignersQueryMsg::GetActiveProposalsPaged {
            start_after,
            limit,
        })
        .await
    }

    async fn get_proposals_paged(
        &self,
        start_after: Option<ProposalId>,
        limit: Option<u32>,
    ) -> Result<ProposalsPagedResponse, NyxdError> {
        self.query_offline_signers_contract(OfflineSignersQueryMsg::GetProposalsPaged {
            start_after,
            limit,
        })
        .await
    }

    async fn get_votes_paged(
        &self,
        proposal: ProposalId,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Result<VotesPagedResponse, NyxdError> {
        self.query_offline_signers_contract(OfflineSignersQueryMsg::GetVotesPaged {
            proposal,
            start_after,
            limit,
        })
        .await
    }

    async fn get_offline_signers_paged(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Result<OfflineSignersPagedResponse, NyxdError> {
        self.query_offline_signers_contract(OfflineSignersQueryMsg::GetOfflineSignersPaged {
            start_after,
            limit,
        })
        .await
    }

    async fn get_last_status_reset_paged(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Result<LastStatusResetPagedResponse, NyxdError> {
        self.query_offline_signers_contract(OfflineSignersQueryMsg::GetLastStatusResetPaged {
            start_after,
            limit,
        })
        .await
    }

    async fn current_signing_status(&self) -> Result<SigningStatusResponse, NyxdError> {
        self.query_offline_signers_contract(OfflineSignersQueryMsg::CurrentSigningStatus {})
            .await
    }

    async fn signing_status_at_height(
        &self,
        block_height: u64,
    ) -> Result<SigningStatusAtHeightResponse, NyxdError> {
        self.query_offline_signers_contract(OfflineSignersQueryMsg::SigningStatusAtHeight {
            block_height,
        })
        .await
    }
}

// extension trait to the query client to deal with the paged queries
// (it didn't feel appropriate to combine it with the existing trait
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait PagedOfflineSignersQueryClient: OfflineSignersQueryClient {
    async fn get_all_active_proposals(&self) -> Result<Vec<ProposalWithResolution>, NyxdError> {
        collect_paged!(self, get_active_proposals_paged, active_proposals)
    }
    async fn get_all_proposals(&self) -> Result<Vec<Proposal>, NyxdError> {
        collect_paged!(self, get_proposals_paged, proposals)
    }
    async fn get_all_votes(&self, proposal: ProposalId) -> Result<Vec<VoteDetails>, NyxdError> {
        collect_paged!(self, get_votes_paged, votes, proposal)
    }
    async fn get_all_offline_signers(&self) -> Result<Vec<OfflineSignerDetails>, NyxdError> {
        collect_paged!(self, get_offline_signers_paged, offline_signers)
    }
    async fn get_all_last_status_reset(&self) -> Result<Vec<LastStatusResetDetails>, NyxdError> {
        collect_paged!(self, get_last_status_reset_paged, status_resets)
    }
}

#[async_trait]
impl<T> PagedOfflineSignersQueryClient for T where T: OfflineSignersQueryClient {}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> OfflineSignersQueryClient for C
where
    C: CosmWasmClient + NymContractsProvider + Send + Sync,
{
    async fn query_offline_signers_contract<T>(
        &self,
        query: OfflineSignersQueryMsg,
    ) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        let offline_signers_contract_address = &self
            .offline_signers_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("offline signers contract"))?;
        self.query_contract_smart(offline_signers_contract_address, &query)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nyxd::contract_traits::tests::IgnoreValue;

    // it's enough that this compiles and clippy is happy about it
    #[allow(dead_code)]
    fn all_query_variants_are_covered<C: OfflineSignersQueryClient + Send + Sync>(
        client: C,
        msg: OfflineSignersQueryMsg,
    ) {
        match msg {
            OfflineSignersQueryMsg::Admin {} => client.admin().ignore(),
            OfflineSignersQueryMsg::GetConfig {} => client.get_config().ignore(),
            OfflineSignersQueryMsg::GetActiveProposal { signer } => {
                client.get_active_proposal(signer.parse().unwrap()).ignore()
            }
            OfflineSignersQueryMsg::GetProposal { proposal_id } => {
                client.get_proposal(proposal_id).ignore()
            }
            OfflineSignersQueryMsg::GetVoteInformation { voter, proposal } => client
                .get_vote_information(voter.parse().unwrap(), proposal)
                .ignore(),
            OfflineSignersQueryMsg::GetOfflineSignerInformation { signer } => client
                .get_offline_signer_information(signer.parse().unwrap())
                .ignore(),
            OfflineSignersQueryMsg::GetOfflineSignersAddressesAtHeight { height } => client
                .get_offline_signers_addresses_at_height(height)
                .ignore(),
            OfflineSignersQueryMsg::GetLastStatusReset { signer } => client
                .get_last_status_reset(signer.parse().unwrap())
                .ignore(),
            OfflineSignersQueryMsg::GetActiveProposalsPaged { start_after, limit } => client
                .get_active_proposals_paged(start_after, limit)
                .ignore(),
            OfflineSignersQueryMsg::GetProposalsPaged { start_after, limit } => {
                client.get_proposals_paged(start_after, limit).ignore()
            }
            OfflineSignersQueryMsg::GetVotesPaged {
                proposal,
                start_after,
                limit,
            } => client
                .get_votes_paged(proposal, start_after, limit)
                .ignore(),
            OfflineSignersQueryMsg::GetOfflineSignersPaged { start_after, limit } => client
                .get_offline_signers_paged(start_after, limit)
                .ignore(),
            OfflineSignersQueryMsg::GetLastStatusResetPaged { start_after, limit } => client
                .get_last_status_reset_paged(start_after, limit)
                .ignore(),
            OfflineSignersQueryMsg::CurrentSigningStatus {} => {
                client.current_signing_status().ignore()
            }
            OfflineSignersQueryMsg::SigningStatusAtHeight { block_height } => {
                client.signing_status_at_height(block_height).ignore()
            }
        };
    }
}

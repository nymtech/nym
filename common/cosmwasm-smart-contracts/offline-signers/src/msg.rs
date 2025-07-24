// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "schema")]
use crate::types::{
    ActiveProposalResponse, ActiveProposalsPagedResponse, LastStatusResetPagedResponse,
    LastStatusResetResponse, OfflineSignerResponse, OfflineSignersAddressesResponse,
    OfflineSignersPagedResponse, ProposalResponse, ProposalsPagedResponse,
    SigningStatusAtHeightResponse, SigningStatusResponse, VoteResponse, VotesPagedResponse,
};
use crate::{Config, ProposalId};
use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct InstantiateMsg {
    /// Address of the DKG contract that's used as the base of the signer information
    pub dkg_contract_address: String,

    #[serde(default)]
    pub config: Config,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Change the admin
    UpdateAdmin { admin: String },

    /// Propose or cast vote on particular DKG signer being offline
    ProposeOrVote { signer: String },

    /// Attempt to reset own offline status
    ResetOfflineStatus {},
}

#[cw_serde]
#[cfg_attr(feature = "schema", derive(cosmwasm_schema::QueryResponses))]
pub enum QueryMsg {
    #[cfg_attr(feature = "schema", returns(cw_controllers::AdminResponse))]
    Admin {},

    /// Returns current config values of the contract
    #[cfg_attr(feature = "schema", returns(Config))]
    GetConfig {},

    /// Returns information of the current active proposal against specific signer
    #[cfg_attr(feature = "schema", returns(ActiveProposalResponse))]
    GetActiveProposal { signer: String },

    /// Returns information about proposal with the specified id
    #[cfg_attr(feature = "schema", returns(ProposalResponse))]
    GetProposal { proposal_id: ProposalId },

    /// Returns information on the vote from the provided voter for the specified proposal
    #[cfg_attr(feature = "schema", returns(VoteResponse))]
    GetVoteInformation { voter: String, proposal: ProposalId },

    /// Returns offline signer information for the provided signer
    #[cfg_attr(feature = "schema", returns(OfflineSignerResponse))]
    GetOfflineSignerInformation { signer: String },

    /// Returns list of addresses of all signers marked as offline at provided height.
    /// If no height is given, the current value is returned instead
    #[cfg_attr(feature = "schema", returns(OfflineSignersAddressesResponse))]
    GetOfflineSignersAddressesAtHeight { height: Option<u64> },

    /// Returns information on the last status reset of the provided signer
    #[cfg_attr(feature = "schema", returns(LastStatusResetResponse))]
    GetLastStatusReset { signer: String },

    /// Returns all (paged) active proposals
    #[cfg_attr(feature = "schema", returns(ActiveProposalsPagedResponse))]
    GetActiveProposalsPaged {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// Returns all (paged) proposals
    #[cfg_attr(feature = "schema", returns(ProposalsPagedResponse))]
    GetProposalsPaged {
        start_after: Option<ProposalId>,
        limit: Option<u32>,
    },

    /// Returns all (paged) votes for the specified proposal
    #[cfg_attr(feature = "schema", returns(VotesPagedResponse))]
    GetVotesPaged {
        proposal: ProposalId,
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// Returns all (paged) offline signers
    #[cfg_attr(feature = "schema", returns(OfflineSignersPagedResponse))]
    GetOfflineSignersPaged {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// Returns all (paged) status resets
    #[cfg_attr(feature = "schema", returns(LastStatusResetPagedResponse))]
    GetLastStatusResetPaged {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// Returns the current signing status, i.e. whether credential issuance is still possible
    #[cfg_attr(feature = "schema", returns(SigningStatusResponse))]
    CurrentSigningStatus {},

    /// Returns the signing status at provided block height, i.e. whether credential issuance was possible at that point
    #[cfg_attr(feature = "schema", returns(SigningStatusAtHeightResponse))]
    SigningStatusAtHeight { block_height: u64 },
}

#[cw_serde]
pub struct MigrateMsg {
    //
}

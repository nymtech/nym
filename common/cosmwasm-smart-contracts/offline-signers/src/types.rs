// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{
    DEFAULT_MAXIMUM_PROPOSAL_LIFETIME_SECS, DEFAULT_REQUIRED_QUORUM,
    DEFAULT_STATUS_CHANGE_COOLDOWN_SECS,
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, BlockInfo, Decimal};

pub type ProposalId = u64;

#[cw_serde]
pub struct Proposal {
    pub created_at: BlockInfo,

    pub id: ProposalId,

    pub proposed_offline_signer: Addr,

    // not strictly necessary but the address of the first sender who has managed to get the message through
    pub proposer: Addr,
}

impl Proposal {
    pub fn expired(&self, current_block: &BlockInfo, lifetime_secs: u64) -> bool {
        self.created_at.time.plus_seconds(lifetime_secs) <= current_block.time
    }
}

#[cw_serde]
pub struct VoteInformation {
    pub voted_at: BlockInfo,
}

impl VoteInformation {
    pub fn new(voted_at: &BlockInfo) -> Self {
        VoteInformation {
            voted_at: voted_at.clone(),
        }
    }
}

#[cw_serde]
pub struct OfflineSignerInformation {
    pub marked_offline_at: BlockInfo,
    pub associated_proposal: ProposalId,
}

impl OfflineSignerInformation {
    pub fn recently_marked_offline(&self, current_block: &BlockInfo, threshold_secs: u64) -> bool {
        self.marked_offline_at.time.plus_seconds(threshold_secs) > current_block.time
    }
}

#[cw_serde]
pub struct StatusResetInformation {
    pub status_reset_at: BlockInfo,
}

impl StatusResetInformation {
    pub fn recently_marked_online(&self, current_block: &BlockInfo, threshold_secs: u64) -> bool {
        self.status_reset_at.time.plus_seconds(threshold_secs) >= current_block.time
    }
}

#[cw_serde]
#[derive(Copy)]
#[serde(default)]
pub struct Config {
    // needed % of eligible voters for a proposal to pass
    pub required_quorum: Decimal,

    // maximum duration (in seconds) a proposal can exist for
    // before its votes are reset if not passed
    pub maximum_proposal_lifetime_secs: u64,

    // minimum time between two consecutive status changes
    // (to prevent signer from going online-offline multiple times a minute)
    pub status_change_cooldown_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            required_quorum: DEFAULT_REQUIRED_QUORUM,
            maximum_proposal_lifetime_secs: DEFAULT_MAXIMUM_PROPOSAL_LIFETIME_SECS,
            status_change_cooldown_secs: DEFAULT_STATUS_CHANGE_COOLDOWN_SECS,
        }
    }
}

#[cw_serde]
pub struct ProposalWithResolution {
    pub proposal: Proposal,
    pub passed: bool,
    pub voting_finished: bool,
}

#[cw_serde]
pub struct ActiveProposalResponse {
    pub proposal: Option<ProposalWithResolution>,
}

#[cw_serde]
pub struct ActiveProposalsPagedResponse {
    pub start_next_after: Option<String>,
    pub active_proposals: Vec<ProposalWithResolution>,
}

#[cw_serde]
pub struct LastStatusResetDetails {
    pub information: StatusResetInformation,
    pub signer: Addr,
}

#[cw_serde]
pub struct LastStatusResetPagedResponse {
    pub start_next_after: Option<String>,
    pub status_resets: Vec<LastStatusResetDetails>,
}

#[cw_serde]
pub struct LastStatusResetResponse {
    pub information: Option<StatusResetInformation>,
}

#[cw_serde]
pub struct OfflineSignerResponse {
    pub information: Option<OfflineSignerInformation>,
}

#[cw_serde]
pub struct OfflineSignersAddressesResponse {
    pub addresses: Vec<Addr>,
}

#[cw_serde]
pub struct OfflineSignerDetails {
    pub information: OfflineSignerInformation,
    pub signer: Addr,
}

#[cw_serde]
pub struct OfflineSignersPagedResponse {
    pub start_next_after: Option<String>,
    pub offline_signers: Vec<OfflineSignerDetails>,
}

#[cw_serde]
pub struct ProposalResponse {
    pub proposal: Option<ProposalWithResolution>,
}

#[cw_serde]
pub struct ProposalsPagedResponse {
    pub start_next_after: Option<ProposalId>,
    pub proposals: Vec<Proposal>,
}

#[cw_serde]
pub struct VoteResponse {
    pub vote: Option<VoteInformation>,
}

#[cw_serde]
pub struct VoteDetails {
    pub voter: Addr,
    pub information: VoteInformation,
}

#[cw_serde]
pub struct VotesPagedResponse {
    pub start_next_after: Option<String>,
    pub votes: Vec<VoteDetails>,
}

#[cw_serde]
pub struct SigningStatusResponse {
    pub dkg_epoch_id: u64,
    pub signing_threshold: u64,
    pub total_group_members: u32,
    pub current_registered_dealers: u32,
    pub offline_signers: u32,
    pub threshold_available: bool,
}

#[cw_serde]
pub struct SigningStatusAtHeightResponse {
    pub block_height: u64,
    pub dkg_epoch_id: u64,
    pub signing_threshold: u64,
    pub current_registered_dealers: u32,
    pub offline_signers: u32,
    pub threshold_available: bool,
}

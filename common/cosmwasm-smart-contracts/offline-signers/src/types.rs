// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

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
    // info on if it was passed, etc,
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

#[cw_serde]
#[derive(Copy)]
pub struct Config {
    // needed % of eligible voters for a proposal to pass
    pub required_quorum: Decimal,

    // maximum duration (in seconds) a proposal can exist for
    // before its votes are reset if not passed
    pub maximum_proposal_lifetime_secs: u64,
}

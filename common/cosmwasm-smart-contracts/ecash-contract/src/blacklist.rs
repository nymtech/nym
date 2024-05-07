// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct BlacklistedAccount {
    public_key: String,
}

impl BlacklistedAccount {
    pub fn new(public_key: String) -> Self {
        BlacklistedAccount { public_key }
    }

    pub fn public_key(&self) -> &str {
        &self.public_key
    }
}

#[cw_serde]
pub struct PagedBlacklistedAccountResponse {
    pub accounts: Vec<BlacklistedAccount>,
    pub per_page: usize,

    /// Field indicating paging information for the following queries if the caller wishes to get further entries.
    pub start_next_after: Option<String>,
}

impl PagedBlacklistedAccountResponse {
    pub fn new(
        accounts: Vec<BlacklistedAccount>,
        per_page: usize,
        start_next_after: Option<String>,
    ) -> Self {
        PagedBlacklistedAccountResponse {
            accounts,
            per_page,
            start_next_after,
        }
    }
}

#[cw_serde]
pub struct BlacklistedAccountResponse {
    pub account: Option<BlacklistedAccount>,
}

impl BlacklistedAccountResponse {
    pub fn new(account: Option<BlacklistedAccount>) -> Self {
        BlacklistedAccountResponse { account }
    }
}

#[cw_serde]
pub struct BlacklistProposal {
    public_key: String,
    proposal_id: u64,
}

impl BlacklistProposal {
    pub fn new(public_key: String, proposal_id: u64) -> Self {
        BlacklistProposal {
            public_key,
            proposal_id,
        }
    }

    pub fn public_key(&self) -> &str {
        &self.public_key
    }

    pub fn proposal_id(&self) -> u64 {
        self.proposal_id
    }
}

#[cw_serde]
pub struct PagedBlacklistProposalResponse {
    pub accounts: Vec<BlacklistProposal>,
    pub per_page: usize,

    /// Field indicating paging information for the following queries if the caller wishes to get further entries.
    pub start_next_after: Option<String>,
}

impl PagedBlacklistProposalResponse {
    pub fn new(
        accounts: Vec<BlacklistProposal>,
        per_page: usize,
        start_next_after: Option<String>,
    ) -> Self {
        PagedBlacklistProposalResponse {
            accounts,
            per_page,
            start_next_after,
        }
    }
}

#[cw_serde]
pub struct BlacklistProposalResponse {
    pub account: Option<BlacklistProposal>,
}

impl BlacklistProposalResponse {
    pub fn new(account: Option<BlacklistProposal>) -> Self {
        BlacklistProposalResponse { account }
    }
}

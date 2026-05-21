// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Blacklist types. **The blacklist surface is stubbed today** - the execute
//! handlers always return `UnimplementedBlacklisting` and the storage map is
//! never populated. These types are kept for the planned redesign.

use cosmwasm_schema::cw_serde;

/// Public-key + metadata pair surfaced by `GetBlacklistedAccount` /
/// `GetBlacklistPaged`. Always empty on a freshly deployed contract.
#[cw_serde]
pub struct BlacklistedAccount {
    pub public_key: String,
    pub info: Blacklisting,
}

impl From<(String, Blacklisting)> for BlacklistedAccount {
    fn from((public_key, info): (String, Blacklisting)) -> Self {
        BlacklistedAccount { public_key, info }
    }
}

/// Per-key blacklist record: the multisig proposal that approved it and the
/// block height at which finalisation landed (None until finalised).
#[cw_serde]
pub struct Blacklisting {
    pub proposal_id: u64,
    pub finalized_at_height: Option<u64>,
}

impl Blacklisting {
    pub fn new(proposal_id: u64) -> Self {
        Blacklisting {
            proposal_id,
            finalized_at_height: None,
        }
    }
}

impl BlacklistedAccount {
    pub fn public_key(&self) -> &str {
        &self.public_key
    }
}

/// Page of blacklist entries returned by `GetBlacklistPaged`. Always empty on
/// a freshly deployed contract.
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

/// Response shape for `GetBlacklistedAccount`. `account` is `None` for any
/// key not present in the (currently always-empty) blacklist.
#[cw_serde]
pub struct BlacklistedAccountResponse {
    pub account: Option<Blacklisting>,
}

impl BlacklistedAccountResponse {
    pub fn new(account: Option<Blacklisting>) -> Self {
        BlacklistedAccountResponse { account }
    }
}

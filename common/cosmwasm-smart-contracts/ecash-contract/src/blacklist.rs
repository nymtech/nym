// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::cw_serde;
use cosmwasm_std::BlockInfo;

#[cw_serde]
pub struct BlacklistedAccount {
    public_key: String,
    height: BlockInfo,
}

impl BlacklistedAccount {
    pub fn new(public_key: String, height: BlockInfo) -> Self {
        BlacklistedAccount { public_key, height }
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

// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin};

/// Whitelist entry: an address and the reduced deposit price it may pay.
/// Persisted in the `"reduced_deposits"` storage map.
#[cw_serde]
pub struct WhitelistedAccount {
    pub address: Addr,
    pub deposit: Coin,
}

/// Response shape for `GetAllWhitelistedAccounts`. Unpaginated - the whitelist
/// is expected to stay small.
#[cw_serde]
pub struct WhitelistedAccountsResponse {
    pub whitelisted_accounts: Vec<WhitelistedAccount>,
}

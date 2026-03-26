// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin};

#[cw_serde]
pub struct WhitelistedAccount {
    pub address: Addr,
    pub deposit: Coin,
}

#[cw_serde]
pub struct WhitelistedAccountsResponse {
    pub whitelisted_accounts: Vec<WhitelistedAccount>,
}

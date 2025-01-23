// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

pub type GranterAddress = Addr;
pub type GranteeAddress = Addr;

#[cw_serde]
pub struct Grant {
    pub granter: GranterAddress,
    pub grantee: GranteeAddress,
    pub allowance: Allowance,
}

#[cw_serde]
pub enum Allowance {
    //
}

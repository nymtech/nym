// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct InstantiateMsg {
    pub pool_denomination: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    //
}

#[cw_serde]
pub enum QueryMsg {
    //
}

#[cw_serde]
pub struct MigrateMsg {
    //
}

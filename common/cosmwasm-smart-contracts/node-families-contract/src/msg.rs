// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct InstantiateMsg {
    //
}

#[cw_serde]
pub enum ExecuteMsg {
    //
}

#[cw_serde]
#[cfg_attr(feature = "schema", derive(cosmwasm_schema::QueryResponses))]
pub enum QueryMsg {
    //
}

#[cw_serde]
pub struct MigrateMsg {
    //
}

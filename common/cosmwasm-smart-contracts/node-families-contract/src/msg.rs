// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use cosmwasm_schema::cw_serde;

/// Message used to instantiate the node families contract.
#[cw_serde]
pub struct InstantiateMsg {
    //
}

/// Execute messages accepted by the contract.
#[cw_serde]
pub enum ExecuteMsg {
    //
}

/// Query messages accepted by the contract.
#[cw_serde]
#[cfg_attr(feature = "schema", derive(cosmwasm_schema::QueryResponses))]
pub enum QueryMsg {
    //
}

/// Message passed to the contract's `migrate` entry point.
#[cw_serde]
pub struct MigrateMsg {
    //
}

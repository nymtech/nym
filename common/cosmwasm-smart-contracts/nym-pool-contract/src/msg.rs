// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::types::{
    AvailableTokensResponse, LockedTokensPagedResponse, LockedTokensResponse,
    TotalLockedTokensResponse,
};
use crate::Allowance;
use cosmwasm_schema::{cw_serde, QueryResponses};
use std::collections::HashMap;

#[cw_serde]
pub struct InstantiateMsg {
    pub pool_denomination: String,

    /// Initial map of grants to be created at instantiation
    pub grants: HashMap<String, Allowance>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Change the admin
    UpdateAdmin { admin: String },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(cw_controllers::AdminResponse)]
    Admin {},

    #[returns(AvailableTokensResponse)]
    GetAvailableTokens {},

    #[returns(TotalLockedTokensResponse)]
    GetTotalLockedTokens {},

    #[returns(LockedTokensResponse)]
    GetLockedTokens { grantee: String },

    #[returns(LockedTokensPagedResponse)]
    GetLockedTokensPaged {
        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,

        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<String>,
    },
}

#[cw_serde]
pub struct MigrateMsg {
    //
}

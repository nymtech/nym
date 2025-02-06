// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::types::{
    AvailableTokensResponse, GrantResponse, GranterResponse, GrantersPagedResponse,
    GrantsPagedResponse, LockedTokensPagedResponse, LockedTokensResponse,
    TotalLockedTokensResponse,
};
use crate::{Allowance, TransferRecipient};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Coin;
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

    /// Attempt to grant new allowance to the specified grantee
    GrantAllowance {
        grantee: String,
        allowance: Box<Allowance>,
    },

    /// Attempt to revoke previously granted allowance
    RevokeAllowance { grantee: String },

    /// Attempt to use allowance
    UseAllowance { recipients: Vec<TransferRecipient> },

    /// Attempt to withdraw the specified amount into the grantee's account
    WithdrawAllowance { amount: Coin },

    /// Attempt to lock part of existing allowance for future use
    LockAllowance { amount: Coin },

    /// Attempt to unlock previously locked allowance
    UnlockAllowance { amount: Coin },

    /// Attempt to use part of the locked allowance
    UseLockedAllowance { recipients: Vec<TransferRecipient> },

    /// Attempt to withdraw the specified amount of locked tokens into the grantee's account
    WithdrawLockedAllowance { amount: Coin },

    /// Attempt to remove expired grant from the storage and unlock (if any) locked tokens
    RemoveExpiredGrant { grantee: String },
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

    #[returns(GrantResponse)]
    GetGrant { grantee: String },

    #[returns(GranterResponse)]
    GetGranter { granter: String },

    #[returns(LockedTokensPagedResponse)]
    GetLockedTokensPaged {
        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,

        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<String>,
    },

    #[returns(GrantersPagedResponse)]
    GetGrantersPaged {
        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,

        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<String>,
    },

    #[returns(GrantsPagedResponse)]
    GetGrantsPaged {
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

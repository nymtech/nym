// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{Allowance, TransferRecipient};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Coin;
use std::collections::HashMap;

#[cfg(feature = "schema")]
use crate::types::{
    AvailableTokensResponse, GrantResponse, GranterResponse, GrantersPagedResponse,
    GrantsPagedResponse, LockedTokensPagedResponse, LockedTokensResponse,
    TotalLockedTokensResponse,
};

#[cw_serde]
pub struct InstantiateMsg {
    pub pool_denomination: String,

    /// Initial map of grants to be created at instantiation
    pub grants: HashMap<String, Allowance>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Change the admin
    UpdateAdmin {
        admin: String,
        // flag to determine whether old admin should be removed from the granter set
        // and new one should be included instead
        // the reason it's provided as an option is to make it possible to skip this field
        // when creating transaction directly with nyxd
        update_granter_set: Option<bool>,
    },

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

    /// Attempt to add a new account to the permitted set of grant granters
    AddNewGranter { granter: String },

    /// Revoke the provided account from the permitted set of granters
    RevokeGranter { granter: String },

    /// Attempt to remove expired grant from the storage and unlock (if any) locked tokens
    RemoveExpiredGrant { grantee: String },
}

#[cw_serde]
#[cfg_attr(feature = "schema", derive(cosmwasm_schema::QueryResponses))]
pub enum QueryMsg {
    #[cfg_attr(feature = "schema", returns(cw_controllers::AdminResponse))]
    Admin {},

    #[cfg_attr(feature = "schema", returns(AvailableTokensResponse))]
    GetAvailableTokens {},

    #[cfg_attr(feature = "schema", returns(TotalLockedTokensResponse))]
    GetTotalLockedTokens {},

    #[cfg_attr(feature = "schema", returns(LockedTokensResponse))]
    GetLockedTokens { grantee: String },

    #[cfg_attr(feature = "schema", returns(GrantResponse))]
    GetGrant { grantee: String },

    #[cfg_attr(feature = "schema", returns(GranterResponse))]
    GetGranter { granter: String },

    #[cfg_attr(feature = "schema", returns(LockedTokensPagedResponse))]
    GetLockedTokensPaged {
        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,

        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<String>,
    },

    #[cfg_attr(feature = "schema", returns(GrantersPagedResponse))]
    GetGrantersPaged {
        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,

        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<String>,
    },

    #[cfg_attr(feature = "schema", returns(GrantsPagedResponse))]
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

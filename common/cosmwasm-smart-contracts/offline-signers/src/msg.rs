// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct InstantiateMsg {
    /// Address of the DKG contract that's used as the base of the signer information
    pub dkg_contract_address: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Change the admin
    UpdateAdmin { admin: String },

    /// Propose or cast vote on particular DKG signer being offline
    ProposeOrVote { signer: String },

    /// Attempt to reset own offline status
    ResetOfflineStatus {},
}

#[cw_serde]
#[cfg_attr(feature = "schema", derive(cosmwasm_schema::QueryResponses))]
pub enum QueryMsg {
    #[cfg_attr(feature = "schema", returns(cw_controllers::AdminResponse))]
    Admin {},
}

#[cw_serde]
pub struct MigrateMsg {
    //
}

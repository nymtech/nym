// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{EpochId, NodePerformance};
use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct InstantiateMsg {
    pub mixnet_contract_address: String,
    pub authorised_network_monitors: Vec<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Change the admin
    UpdateAdmin { admin: String },

    /// Attempt to submit performance data of a particular node for given epoch
    Submit {
        epoch: EpochId,
        data: NodePerformance,
    },

    /// Attempt to submit performance data of a batch of nodes for given epoch
    BatchSubmit {
        epoch: EpochId,
        data: Vec<NodePerformance>,
    },

    /// Attempt to authorise new network monitor for submitting performance data
    AuthoriseNetworkMonitor { address: String },

    /// Attempt to retire an existing network monitor and forbid it from submitting any future performance data
    RetireNetworkMonitor { address: String },
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

// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::cw_serde;
use std::net::SocketAddr;

#[cfg(feature = "schema")]
use crate::{
    AuthorisedNetworkMonitorOrchestratorsResponse, AuthorisedNetworkMonitorsPagedResponse,
};

#[cw_serde]
pub struct InstantiateMsg {
    /// Address of the initial network monitor orchestrator.
    pub orchestrator_address: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Change the admin
    UpdateAdmin { admin: String },

    /// Authorise new network monitor orchestrator
    AuthoriseNetworkMonitorOrchestrator { address: String },

    /// Attempt to update the announced identity key of this orchestrator
    UpdateOrchestratorIdentityKey { key: String },

    /// Revoke network monitor orchestrator authorisation.
    RevokeNetworkMonitorOrchestrator { address: String },

    /// Authorise new network monitor (or renew authorisation)
    /// granting additional privileges when sending mixnet packets to Nym nodes.
    AuthoriseNetworkMonitor {
        /// Mixnet address of the agent.
        /// The underlying ip address is going to be used as ingress to the nodes,
        /// and the full socket address announces the egress and the association with the noise key
        mixnet_address: SocketAddr,

        /// Base-58 encoded noise key of the agent.
        bs58_x25519_noise: String,

        /// Version of the noise protocol used by the agent.
        noise_version: u8,
    },

    /// Revoke network monitor authorisation.
    RevokeNetworkMonitor { address: SocketAddr },

    /// Revoke all network monitor authorisations.
    RevokeAllNetworkMonitors,
}

#[cw_serde]
#[cfg_attr(feature = "schema", derive(cosmwasm_schema::QueryResponses))]
pub enum QueryMsg {
    #[cfg_attr(feature = "schema", returns(cw_controllers::AdminResponse))]
    Admin {},

    // no need for pagination as we don't expect even a double digit of those
    #[cfg_attr(
        feature = "schema",
        returns(AuthorisedNetworkMonitorOrchestratorsResponse)
    )]
    NetworkMonitorOrchestrators {},

    #[cfg_attr(feature = "schema", returns(AuthorisedNetworkMonitorsPagedResponse))]
    NetworkMonitorAgents {
        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_next_after: Option<SocketAddr>,

        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct MigrateMsg {}

// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Timestamp};
use std::net::IpAddr;

pub type OrchestratorAddress = Addr;

// Unfortunately `IpAddr` does not implement `PrimaryKey`, so we store its stringified representation instead
pub type NetworkMonitorAddress = String;

#[cw_serde]
pub struct AuthorisedNetworkMonitorOrchestrator {
    /// The address associated with the network monitor orchestrator.
    pub address: Addr,

    /// Timestamp of when the network monitor was authorised.
    pub authorised_at: Timestamp,
}

#[cw_serde]
pub struct AuthorisedNetworkMonitor {
    /// The Ip address associated with the network monitor agent.
    pub address: IpAddr,

    /// The address of the orchestrator that authorised the network monitor agent.
    pub authorised_by: OrchestratorAddress,

    /// Timestamp of when the network monitor was authorised.
    pub authorised_at: Timestamp,

    /// Base-58 encoded noise key of the agent.
    pub bs58_x25519_noise: String,

    /// Version of the noise protocol used by the agent.
    pub noise_version: u8,
}

#[cw_serde]
pub struct AuthorisedNetworkMonitorOrchestratorsResponse {
    pub authorised: Vec<AuthorisedNetworkMonitorOrchestrator>,
}

#[cw_serde]
pub struct AuthorisedNetworkMonitorsPagedResponse {
    pub authorised: Vec<AuthorisedNetworkMonitor>,

    pub start_next_after: Option<IpAddr>,
}

// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Timestamp};
use std::net::SocketAddr;

pub type OrchestratorAddress = Addr;

#[cw_serde]
pub struct AuthorisedNetworkMonitorOrchestrator {
    /// The address associated with the network monitor orchestrator.
    pub address: Addr,

    /// Base-58 encoded identity key of the orchestrator, announced by the orchestrator itself
    /// on startup.
    pub identity_key: Option<String>,

    /// Timestamp of when the network monitor was authorised.
    pub authorised_at: Timestamp,
}

#[cw_serde]
pub struct AuthorisedNetworkMonitor {
    /// Mixnet address of the agent.
    /// The underlying ip address is going to be used as ingress to the nodes,
    /// and the full socket address announces the egress and the association with the noise key
    pub mixnet_address: SocketAddr,

    /// The address of the orchestrator that authorised the network monitor agent.
    pub authorised_by: OrchestratorAddress,

    /// Timestamp of when the network monitor was authorised.
    pub authorised_at: Timestamp,

    /// Base-58 encoded noise key of the agent.
    pub bs58_x25519_noise: String,

    /// Version of the noise protocol used by the agent.
    pub noise_version: u8,

    /// Base-58 encoded ed25519 identity key of the agent.
    /// `None` for entries saved before the field existed; the upsert populates it on re-announcement.
    pub bs58_ed25519_identity: Option<String>,
}

#[cw_serde]
pub struct AuthorisedNetworkMonitorOrchestratorsResponse {
    pub authorised: Vec<AuthorisedNetworkMonitorOrchestrator>,
}

#[cw_serde]
pub struct AuthorisedNetworkMonitorsPagedResponse {
    pub authorised: Vec<AuthorisedNetworkMonitor>,

    pub start_next_after: Option<SocketAddr>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::from_json;

    #[test]
    fn agent_entry_saved_before_the_identity_field_loads_with_none() {
        // exactly what the contract wrote before `bs58_ed25519_identity` existed, which is why
        // the migration needs no backfill
        let stored = r#"{
            "mixnet_address": "1.1.1.1:1789",
            "authorised_by": "n1foomp",
            "authorised_at": "1700000000000000000",
            "bs58_x25519_noise": "11111111111111111111111111111111",
            "noise_version": 1
        }"#;

        let recovered: AuthorisedNetworkMonitor = from_json(stored).unwrap();
        assert!(recovered.bs58_ed25519_identity.is_none());
        assert_eq!(recovered.noise_version, 1);
    }
}

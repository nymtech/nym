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

        /// Base-58 encoded ed25519 identity key of the agent, if it announced one.
        bs58_ed25519_identity: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{from_json, to_json_vec};

    /// The `AuthoriseNetworkMonitor` payload as a consumer compiled before `bs58_ed25519_identity`
    /// existed sees it. Nym nodes learn about agents by deserialising this message out of a
    /// transaction, and treat a parse failure as non-fatal: they log and continue to the next
    /// block. An un-upgraded node that could not parse the new form would therefore silently stop
    /// applying authorisations and revocations, so this compatibility is load-bearing.
    #[cw_serde]
    enum LegacyExecuteMsg {
        AuthoriseNetworkMonitor {
            mixnet_address: SocketAddr,
            bs58_x25519_noise: String,
            noise_version: u8,
        },
    }

    // the other direction, which is what an UNMIGRATED deployment does: an orchestrator that predates
    // the field authorises an agent without one, and a node compiled with the field has to apply that
    // authorisation rather than log a parse failure and skip it. a node that could not read the
    // legacy form would silently stop learning about agents the moment it was upgraded, before any
    // contract migration had happened
    #[test]
    fn authorisation_carrying_no_identity_still_parses_under_the_current_schema() {
        let legacy = LegacyExecuteMsg::AuthoriseNetworkMonitor {
            mixnet_address: "1.1.1.1:1789".parse().unwrap(),
            bs58_x25519_noise: "11111111111111111111111111111111".to_string(),
            noise_version: 1,
        };

        let current: ExecuteMsg = from_json(to_json_vec(&legacy).unwrap()).unwrap();

        let ExecuteMsg::AuthoriseNetworkMonitor {
            mixnet_address,
            bs58_ed25519_identity,
            ..
        } = current
        else {
            panic!("a legacy authorisation did not parse as one")
        };

        assert_eq!(mixnet_address, "1.1.1.1:1789".parse().unwrap());
        // absent rather than rejected, which is what lets the node apply the authorisation and simply
        // not recognise the agent on the client-session path
        assert!(bs58_ed25519_identity.is_none());
    }

    #[test]
    fn authorisation_carrying_an_identity_still_parses_under_the_legacy_schema() {
        let current = ExecuteMsg::AuthoriseNetworkMonitor {
            mixnet_address: "1.1.1.1:1789".parse().unwrap(),
            bs58_x25519_noise: "11111111111111111111111111111111".to_string(),
            noise_version: 1,
            bs58_ed25519_identity: Some("22222222222222222222222222222222".to_string()),
        };

        let legacy: LegacyExecuteMsg = from_json(to_json_vec(&current).unwrap()).unwrap();

        let LegacyExecuteMsg::AuthoriseNetworkMonitor {
            mixnet_address,
            bs58_x25519_noise,
            noise_version,
        } = legacy;

        assert_eq!(
            mixnet_address,
            "1.1.1.1:1789".parse::<SocketAddr>().unwrap()
        );
        assert_eq!(bs58_x25519_noise, "11111111111111111111111111111111");
        assert_eq!(noise_version, 1);
    }
}

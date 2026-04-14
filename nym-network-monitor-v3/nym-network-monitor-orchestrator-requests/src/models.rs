// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_crypto::asymmetric::x25519;
use nym_crypto::asymmetric::x25519::serde_helpers::bs58_x25519_pubkey;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
/// Body sent by an agent to announce its details to the orchestrator.
/// The orchestrator forwards this information to the smart contract so that
/// network nodes can whitelist connections from known agents.
pub struct AgentAnnounceRequest {
    /// Egress address of the agent node
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub agent_node_ip: IpAddr,

    /// Port the agent will use for opening mixnet socket.
    /// It is deterministically derived from its pod ip
    pub agent_mixnet_socket_port: u16,

    /// Base-58 encoded noise key of the agent.
    #[serde(with = "bs58_x25519_pubkey")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub x25519_noise_key: x25519::PublicKey,

    /// Version of the noise protocol used by the agent.
    pub noise_version: u8,
}

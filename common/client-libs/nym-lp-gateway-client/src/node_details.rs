// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Turning what the directory publishes about a node into what it takes to reach it over LP.
//!
//! Two things have to be inferred rather than read. The node does not advertise its LP protocol
//! version or its KKT ciphersuite; both are derived from its build version, the same way and in the
//! same place, so that a caller cannot get one right and the other wrong.

use crate::error::LpClientError;
use nym_lp::Ciphersuite;
use nym_lp::peer::LpRemotePeer;
use nym_lp_data::packet::version;
use nym_topology::lp::LpRoutingNode;
use std::net::SocketAddr;

/// What it takes to open an LP channel to a node.
/// In particular, resolves ciphersuite and protocol version from the routing node
#[derive(Clone, Debug)]
pub struct LpConnectionDetails {
    /// TCP, for the handshake and anything expecting an answer.
    pub control_address: SocketAddr,

    /// UDP, for data frames.
    pub data_address: SocketAddr,

    /// The node's LP identity: its x25519 key and the KEM key digests it will accept.
    pub peer: LpRemotePeer,

    pub ciphersuite: Ciphersuite,

    /// What the node speaks, before [`version::negotiate`] has a say.
    pub protocol_version: u8,
}

impl TryFrom<&LpRoutingNode> for LpConnectionDetails {
    type Error = LpClientError;

    fn try_from(value: &LpRoutingNode) -> Result<Self, Self::Error> {
        let build_version = value.build_version.clone();
        let ciphersuite =
            Ciphersuite::from_node_version(build_version.clone()).ok_or_else(|| {
                LpClientError::NoLpForBuildVersion {
                    build_version: build_version.to_string(),
                }
            })?;

        let protocol_version =
            version::from_node_version(build_version.clone()).ok_or_else(|| {
                LpClientError::NoLpForBuildVersion {
                    build_version: build_version.to_string(),
                }
            })?;

        Ok(LpConnectionDetails {
            control_address: value.control_host,
            data_address: value.data_host,
            peer: value.remote_peer.clone(),
            ciphersuite,
            protocol_version,
        })
    }
}

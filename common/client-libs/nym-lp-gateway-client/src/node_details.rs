// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Turning what the directory publishes about a node into what it takes to reach it over LP.
//!
//! Two things have to be inferred rather than read. The node does not advertise its LP protocol
//! version or its KKT ciphersuite; both are derived from its build version, the same way and in the
//! same place, so that a caller cannot get one right and the other wrong.

use crate::error::{LpClientError, Result};
use nym_api_requests::models::described::type_translation::LewesProtocolDetailsDataV1;
use nym_lp::Ciphersuite;
use nym_lp::peer::LpRemotePeer;
use nym_lp_data::packet::version;
use std::net::{IpAddr, SocketAddr};

/// What it takes to open an LP channel to a node.
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

impl LpConnectionDetails {
    /// Resolve a node's published LP details against one of its addresses.
    ///
    /// `ip` picks which of the node's addresses to reach it on; the ports come from what it
    /// published. Fails if the node has LP switched off, published malformed key digests, or runs
    /// a build too old to speak LP at all.
    pub fn resolve(
        published: &LewesProtocolDetailsDataV1,
        ip: IpAddr,
        build_version: &semver::Version,
    ) -> Result<Self> {
        if !published.enabled {
            return Err(LpClientError::LpNotEnabled);
        }

        let kem_key_digests = published
            .kem_keys()
            .map_err(|source| LpClientError::MalformedLpNodeDetails { source })?;

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
            control_address: SocketAddr::new(ip, published.control_port),
            data_address: SocketAddr::new(ip, published.data_port),
            peer: LpRemotePeer::new(published.x25519).with_key_digests(kem_key_digests),
            ciphersuite,
            protocol_version,
        })
    }
}

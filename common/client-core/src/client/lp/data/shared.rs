// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use nym_client_core_config_types::DebugConfig;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_lp::LpTransportSession;
use nym_lp_data::fragmentation::reconstruction::MessageReconstructor;
use nym_task::ShutdownToken;
use nym_topology::NodeId;

/// An established LP session with one gateway, and where its data packets go.
///
/// Both halves are here rather than with the transport, which holds neither: it moves packets
/// someone else has encrypted, to an address someone else names.
pub struct LpGatewaySession {
    /// Named by the gateway during registration, so it can be addressed as well as read.
    pub session: LpTransportSession,

    /// The gateway's UDP address, not the TCP one the handshake ran on.
    pub data_address: SocketAddr,
}

/// Shared state for LP data plane
pub struct SharedLpDataState {
    pub(crate) config: DebugConfig,

    pub(crate) encryption_keys: Arc<x25519::KeyPair>,
    pub(crate) identity_keys: Arc<ed25519::KeyPair>,

    /// The sessions outbound packets are encrypted on.
    ///
    /// One entry today. Keyed from the start so that reaching a second gateway is adding an entry
    /// rather than reshaping this.
    pub(crate) gateway_sessions: HashMap<NodeId, LpGatewaySession>,

    pub(crate) message_reconstructor: MessageReconstructor,

    pub(crate) shutdown_token: ShutdownToken,
}

impl SharedLpDataState {
    pub(crate) fn new(
        config: DebugConfig,
        encryption_keys: Arc<x25519::KeyPair>,
        identity_keys: Arc<ed25519::KeyPair>,
        gateway_sessions: HashMap<NodeId, LpGatewaySession>,
        shutdown_token: ShutdownToken,
    ) -> Self {
        SharedLpDataState {
            config,
            encryption_keys,
            identity_keys,
            gateway_sessions,
            message_reconstructor: Default::default(),
            shutdown_token,
        }
    }
}

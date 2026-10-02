// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The client's LP control plane: how a session with a gateway comes to exist.
//!
//! Its whole job is dialing, so the plane is one [`LpGatewayDialer`] and the state it needs. There
//! is no listener: a client is not dialled, it dials - a gateway reaches it over the session it
//! registered, not over a connection of its own.

use std::sync::Arc;

use nym_crypto::asymmetric::ed25519;
use nym_task::ShutdownToken;
use tokio::net::TcpStream;

use crate::client::lp::data::shared::LpGatewaySessions;
use crate::client::topology_control::TopologyAccessor;
use crate::config::LewesProtocol;

pub mod dialer;
pub mod error;

pub use dialer::LpGatewayDialer;
pub use error::LpControlError;

/// The control plane, built once and handed out by the handle.
///
/// Construct-then-consume like the node's `LpControlSetup`, minus the consuming: a dial spawns its
/// own task, so there is nothing here that needs starting. What it owns is the wiring - which
/// topology to resolve against, whose keys to handshake with, where a finished session is filed -
/// so that everything asking for a session asks the same thing.
pub(crate) struct LpClientControlSetup<S = TcpStream> {
    dialer: LpGatewayDialer<S>,
}

impl LpClientControlSetup {
    pub(crate) fn new(
        config: &LewesProtocol,
        topology_accessor: TopologyAccessor,
        sessions: LpGatewaySessions,
        identity_keys: Arc<ed25519::KeyPair>,
        shutdown: ShutdownToken,
    ) -> Self {
        LpClientControlSetup {
            dialer: LpGatewayDialer::new(
                topology_accessor,
                sessions,
                identity_keys,
                config,
                shutdown,
            ),
        }
    }

    /// Handle for asking for a session with a gateway to be established.
    pub(crate) fn dialer(&self) -> LpGatewayDialer {
        self.dialer.clone()
    }
}

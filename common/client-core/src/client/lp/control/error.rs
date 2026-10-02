// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

/// Why the control plane could not produce a session.
///
/// `Clone` because one dial's outcome is published to every caller waiting on that gateway, and
/// that is also what rules out carrying the errors this is built from: none of
/// [`NymTopologyError`], [`LpClientError`] or [`ClientCoreError`] is `Clone`. What they said is
/// kept as text, and the dial that failed logs the whole thing where it happened.
///
/// Names no gateway either: every caller already knows which one it asked about, and an ed25519
/// key is several hundred bytes to carry in an error that is cloned to every waiter.
///
/// [`NymTopologyError`]: nym_topology::NymTopologyError
/// [`LpClientError`]: nym_lp_gateway_client::LpClientError
/// [`ClientCoreError`]: crate::error::ClientCoreError
#[derive(Debug, Clone, Error)]
pub enum LpControlError {
    #[error("nothing says how to reach that gateway over LP: {0}")]
    UnreachableGateway(String),

    #[error("the LP handshake with that gateway failed: {0}")]
    HandshakeFailed(String),

    #[error("that gateway would not register us: {0}")]
    RegistrationFailed(String),

    #[error("the client is shutting down")]
    ShuttingDown,
}

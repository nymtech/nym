// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GatewayWireguardError {
    #[error("internal error: {0}")]
    InternalError(String),

    #[error("peers can't be interacted with anymore")]
    PeerInteractionStopped,
}

impl GatewayWireguardError {
    pub fn internal(message: impl Into<String>) -> Self {
        GatewayWireguardError::InternalError(message.into())
    }
}

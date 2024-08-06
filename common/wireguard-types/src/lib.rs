// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod config;
pub mod error;
pub mod public_key;
pub mod registration;

use std::time::Duration;

pub use config::Config;
pub use error::Error;
pub use public_key::PeerPublicKey;
pub use registration::{
    ClientMac, ClientMessage, GatewayClient, GatewayClientRegistry, InitMessage, Nonce,
};

// To avoid any problems, keep this stale check time bigger (>2x) then the bandwidth cap
// reset time (currently that one is 24h, at UTC midnight)
pub const DEFAULT_PEER_TIMEOUT: Duration = Duration::from_secs(60 * 60 * 24 * 3); // 3 days
pub const DEFAULT_PEER_TIMEOUT_CHECK: Duration = Duration::from_secs(5); // 5 seconds

#[cfg(feature = "verify")]
pub use registration::HmacSha256;

// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod config;
pub mod error;
pub mod public_key;

use std::time::Duration;

pub use config::Config;
pub use error::Error;
pub use public_key::PeerPublicKey;

pub const DEFAULT_PEER_TIMEOUT_CHECK: Duration = Duration::from_secs(5); // 5 seconds

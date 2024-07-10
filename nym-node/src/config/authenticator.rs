// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_client_core_config_types::DebugConfig as ClientDebugConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct Authenticator {
    #[serde(default)]
    pub debug: AuthenticatorDebug,
}

#[allow(clippy::derivable_impls)]
impl Default for Authenticator {
    fn default() -> Self {
        Authenticator {
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct AuthenticatorDebug {
    /// Specifies whether authenticator service is enabled in this process.
    /// This is only here for debugging purposes as exit gateway should always run
    /// the authenticator.
    pub enabled: bool,

    /// Disable Poisson sending rate.
    /// This is equivalent to setting client_debug.traffic.disable_main_poisson_packet_distribution = true
    /// (or is it (?))
    pub disable_poisson_rate: bool,

    /// Shared detailed client configuration options
    #[serde(flatten)]
    pub client_debug: ClientDebugConfig,
}

impl Default for AuthenticatorDebug {
    fn default() -> Self {
        AuthenticatorDebug {
            enabled: true,
            disable_poisson_rate: true,
            client_debug: Default::default(),
        }
    }
}

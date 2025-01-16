// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::authenticator::Authenticator;
use crate::config::persistence::ServiceProvidersPaths;
use nym_client_core_config_types::DebugConfig as ClientDebugConfig;
use nym_config::defaults::mainnet;
use serde::{Deserialize, Serialize};
use std::path::Path;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceProvidersConfig {
    pub storage_paths: ServiceProvidersPaths,

    /// specifies whether this exit node should run in 'open-proxy' mode
    /// and thus would attempt to resolve **ANY** request it receives.
    pub open_proxy: bool,

    /// Specifies the url for an upstream source of the exit policy used by this node.
    pub upstream_exit_policy_url: Url,

    pub network_requester: NetworkRequester,

    pub ip_packet_router: IpPacketRouter,

    pub authenticator: Authenticator,

    #[serde(default)]
    pub debug: Debug,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Debug {
    /// Number of messages from offline client that can be pulled at once (i.e. with a single SQL query) from the storage.
    pub message_retrieval_limit: i64,
}

impl Debug {
    const DEFAULT_MESSAGE_RETRIEVAL_LIMIT: i64 = 100;
}

impl Default for Debug {
    fn default() -> Self {
        Debug {
            message_retrieval_limit: Self::DEFAULT_MESSAGE_RETRIEVAL_LIMIT,
        }
    }
}

impl ServiceProvidersConfig {
    pub fn new_default<P: AsRef<Path>>(data_dir: P) -> Self {
        #[allow(clippy::expect_used)]
        // SAFETY:
        // we expect our default values to be well-formed
        ServiceProvidersConfig {
            storage_paths: ServiceProvidersPaths::new(data_dir),
            open_proxy: false,
            upstream_exit_policy_url: mainnet::EXIT_POLICY_URL
                .parse()
                .expect("invalid default exit policy URL"),
            network_requester: Default::default(),
            ip_packet_router: Default::default(),
            authenticator: Default::default(),
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
pub struct NetworkRequester {
    #[serde(default)]
    pub debug: NetworkRequesterDebug,
}

#[allow(clippy::derivable_impls)]
impl Default for NetworkRequester {
    fn default() -> Self {
        NetworkRequester {
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
pub struct NetworkRequesterDebug {
    /// Specifies whether network requester service is enabled in this process.
    /// This is only here for debugging purposes as exit gateway should always run **both**
    /// network requester and an ip packet router.
    pub enabled: bool,

    /// Disable Poisson sending rate.
    /// This is equivalent to setting client_debug.traffic.disable_main_poisson_packet_distribution = true
    /// (or is it (?))
    pub disable_poisson_rate: bool,

    /// Shared detailed client configuration options
    #[serde(flatten)]
    pub client_debug: ClientDebugConfig,
}

impl Default for NetworkRequesterDebug {
    fn default() -> Self {
        NetworkRequesterDebug {
            enabled: true,
            disable_poisson_rate: true,
            client_debug: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct IpPacketRouter {
    #[serde(default)]
    pub debug: IpPacketRouterDebug,
}

#[allow(clippy::derivable_impls)]
impl Default for IpPacketRouter {
    fn default() -> Self {
        IpPacketRouter {
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct IpPacketRouterDebug {
    /// Specifies whether ip packet routing service is enabled in this process.
    /// This is only here for debugging purposes as exit gateway should always run **both**
    /// network requester and an ip packet router.
    pub enabled: bool,

    /// Disable Poisson sending rate.
    /// This is equivalent to setting client_debug.traffic.disable_main_poisson_packet_distribution = true
    /// (or is it (?))
    pub disable_poisson_rate: bool,

    /// Shared detailed client configuration options
    #[serde(flatten)]
    pub client_debug: ClientDebugConfig,
}

impl Default for IpPacketRouterDebug {
    fn default() -> Self {
        IpPacketRouterDebug {
            enabled: true,
            disable_poisson_rate: true,
            client_debug: Default::default(),
        }
    }
}

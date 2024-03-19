// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::persistence::ExitGatewayPaths;
use crate::config::Config;
use crate::error::ExitGatewayError;
use nym_client_core_config_types::DebugConfig as ClientDebugConfig;
use nym_config::serde_helpers::de_maybe_stringified;
use nym_gateway::node::{LocalIpPacketRouterOpts, LocalNetworkRequesterOpts};
use serde::{Deserialize, Serialize};
use std::path::Path;
use url::Url;
use zeroize::Zeroizing;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExitGatewayConfig {
    pub storage_paths: ExitGatewayPaths,

    /// specifies whether this exit node should run in 'open-proxy' mode
    /// and thus would attempt to resolve **ANY** request it receives.
    pub open_proxy: bool,

    /// Specifies the url for an upstream source of the exit policy used by this node.
    #[serde(deserialize_with = "de_maybe_stringified")]
    pub upstream_exit_policy_url: Option<Url>,

    pub network_requester: NetworkRequester,

    pub ip_packet_router: IpPacketRouter,
}

impl ExitGatewayConfig {
    pub fn new_default<P: AsRef<Path>>(data_dir: P) -> Self {
        todo!()
        // ExitGatewayConfig {
        //     storage_paths: ExitGatewayPaths::new(data_dir),
        //     network_requester: Default::default(),
        //     ip_packet_router: Default::default(),
        // }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct NetworkRequester {
    #[serde(default)]
    pub debug: NetworkRequesterDebug,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
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
        todo!()
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct IpPacketRouter {
    #[serde(default)]
    pub debug: IpPacketRouterDebug,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
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
        todo!()
    }
}

pub struct EphemeralConfig {
    pub gateway: nym_gateway::config::Config,
    pub nr_opts: LocalNetworkRequesterOpts,
    pub ipr_opts: LocalIpPacketRouterOpts,
}

pub fn ephemeral_exit_gateway_config(
    config: Config,
    mnemonic: Zeroizing<bip39::Mnemonic>,
) -> Result<EphemeralConfig, ExitGatewayError> {
    todo!()
}

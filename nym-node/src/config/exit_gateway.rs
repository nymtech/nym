// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::helpers::ephemeral_gateway_config;
use crate::config::persistence::ExitGatewayPaths;
use crate::config::Config;
use crate::error::ExitGatewayError;
use nym_client_core_config_types::DebugConfig as ClientDebugConfig;
use nym_config::defaults::mainnet;
use nym_gateway::node::{
    LocalAuthenticatorOpts, LocalIpPacketRouterOpts, LocalNetworkRequesterOpts,
};
use serde::{Deserialize, Serialize};
use std::{path::Path, str::FromStr};
use url::Url;

use super::{
    helpers::{base_client_config, EphemeralConfig},
    LocalWireguardOpts,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExitGatewayConfig {
    pub storage_paths: ExitGatewayPaths,

    /// specifies whether this exit node should run in 'open-proxy' mode
    /// and thus would attempt to resolve **ANY** request it receives.
    pub open_proxy: bool,

    /// Specifies the url for an upstream source of the exit policy used by this node.
    pub upstream_exit_policy_url: Url,

    pub network_requester: NetworkRequester,

    pub ip_packet_router: IpPacketRouter,

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

impl ExitGatewayConfig {
    pub fn new_default<P: AsRef<Path>>(data_dir: P) -> Self {
        #[allow(clippy::expect_used)]
        // SAFETY:
        // we expect our default values to be well-formed
        ExitGatewayConfig {
            storage_paths: ExitGatewayPaths::new(data_dir),
            open_proxy: false,
            upstream_exit_policy_url: mainnet::EXIT_POLICY_URL
                .parse()
                .expect("invalid default exit policy URL"),
            network_requester: Default::default(),
            ip_packet_router: Default::default(),
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

// that function is rather disgusting, but I hope it's not going to live for too long
pub fn ephemeral_exit_gateway_config(
    config: Config,
    mnemonic: &bip39::Mnemonic,
) -> Result<EphemeralConfig, ExitGatewayError> {
    let mut nr_opts = LocalNetworkRequesterOpts {
        config: nym_network_requester::Config {
            base: nym_client_core_config_types::Config {
                client: base_client_config(&config),
                debug: config.exit_gateway.network_requester.debug.client_debug,
            },
            network_requester: nym_network_requester::config::NetworkRequester {
                open_proxy: config.exit_gateway.open_proxy,
                disable_poisson_rate: config
                    .exit_gateway
                    .network_requester
                    .debug
                    .disable_poisson_rate,
                upstream_exit_policy_url: Some(
                    config.exit_gateway.upstream_exit_policy_url.clone(),
                ),
            },
            storage_paths: nym_network_requester::config::NetworkRequesterPaths {
                common_paths: config
                    .exit_gateway
                    .storage_paths
                    .network_requester
                    .to_common_client_paths(),
            },
            network_requester_debug: Default::default(),
            logging: config.logging,
        },
        custom_mixnet_path: None,
    };

    // SAFETY: this function can only fail if fastmode or nocover is set alongside medium_toggle which is not the case here
    #[allow(clippy::unwrap_used)]
    nr_opts
        .config
        .base
        .try_apply_traffic_modes(
            nr_opts.config.network_requester.disable_poisson_rate,
            false,
            false,
            false,
        )
        .unwrap();

    let mut ipr_opts = LocalIpPacketRouterOpts {
        config: nym_ip_packet_router::Config {
            base: nym_client_core_config_types::Config {
                client: base_client_config(&config),
                debug: config.exit_gateway.ip_packet_router.debug.client_debug,
            },
            ip_packet_router: nym_ip_packet_router::config::IpPacketRouter {
                disable_poisson_rate: config
                    .exit_gateway
                    .ip_packet_router
                    .debug
                    .disable_poisson_rate,
                upstream_exit_policy_url: Some(
                    config.exit_gateway.upstream_exit_policy_url.clone(),
                ),
            },
            storage_paths: nym_ip_packet_router::config::IpPacketRouterPaths {
                common_paths: config
                    .exit_gateway
                    .storage_paths
                    .ip_packet_router
                    .to_common_client_paths(),
                ip_packet_router_description: Default::default(),
            },

            logging: config.logging,
        },
        custom_mixnet_path: std::path::PathBuf::from_str("/tmp/topology2").ok(),
    };

    if ipr_opts.config.ip_packet_router.disable_poisson_rate {
        ipr_opts.config.base.set_no_poisson_process()
    }

    let auth_opts = LocalAuthenticatorOpts {
        config: nym_authenticator::Config {
            base: nym_client_core_config_types::Config {
                client: base_client_config(&config),
                debug: config.authenticator.debug.client_debug,
            },
            authenticator: config.wireguard.clone().into(),
            storage_paths: nym_authenticator::config::AuthenticatorPaths {
                common_paths: config
                    .exit_gateway
                    .storage_paths
                    .authenticator
                    .to_common_client_paths(),
            },
            logging: config.logging,
        },
        custom_mixnet_path: None,
    };

    let pub_id_path = config
        .storage_paths
        .keys
        .public_ed25519_identity_key_file
        .clone();
    let ipr_enabled = config.exit_gateway.ip_packet_router.debug.enabled;
    let nr_enabled = config.exit_gateway.network_requester.debug.enabled;

    let wg_opts = LocalWireguardOpts {
        config: super::Wireguard {
            enabled: config.wireguard.enabled,
            bind_address: config.wireguard.bind_address,
            private_ip: config.wireguard.private_ip,
            announced_port: config.wireguard.announced_port,
            private_network_prefix: config.wireguard.private_network_prefix,
            storage_paths: config.wireguard.storage_paths.clone(),
        },
        custom_mixnet_path: None,
    };

    let mut gateway = ephemeral_gateway_config(config, mnemonic)?;
    gateway.ip_packet_router.enabled = ipr_enabled;
    gateway.network_requester.enabled = nr_enabled;

    // this is temporary until http api is fully managed by nymnode itself
    // (because currently gateway is loading its public key for the second time when starting the API to determine addresses of its clients.
    // Obviously this doesn't work properly without the valid paths)
    gateway.storage_paths.keys.public_identity_key_file = pub_id_path;

    Ok(EphemeralConfig {
        nr_opts: Some(nr_opts),
        ipr_opts: Some(ipr_opts),
        auth_opts,
        wg_opts,
        gateway,
    })
}

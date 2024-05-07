// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::DEFAULT_NYMNODE_ID;
use crate::env::vars::*;
use celes::Country;
use clap::builder::ArgPredicate;
use clap::Args;
use nym_node::config;
use nym_node::config::default_config_filepath;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use url::Url;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Args, Debug)]
pub(crate) struct ConfigArgs {
    /// Id of the nym-node to use
    #[clap(
        long,
        default_value = DEFAULT_NYMNODE_ID,
        default_value_if("config_file", ArgPredicate::IsPresent, None),
        env = NYMNODE_ID_ARG,
        group = "config"
    )]
    id: Option<String>,

    /// Path to a configuration file of this node.
    #[clap(
        long,
        env = NYMNODE_CONFIG_PATH_ARG,
        group = "config"
    )]
    config_file: Option<PathBuf>,
}

impl ConfigArgs {
    pub(crate) fn id(&self) -> &Option<String> {
        &self.id
    }

    pub(crate) fn config_path(&self) -> PathBuf {
        // SAFETY:
        // if `config_file` hasn't been specified, `id` will default to "DEFAULT_NYMNODE_ID",
        // so some value will always be available to use
        #[allow(clippy::unwrap_used)]
        self.config_file
            .clone()
            .unwrap_or_else(|| default_config_filepath(self.id.as_ref().unwrap()))
    }
}

#[derive(clap::Args, Debug)]
pub(crate) struct HostArgs {
    /// Comma separated list of public ip addresses that will be announced to the nym-api and subsequently to the clients.
    /// In nearly all circumstances, it's going to be identical to the address you're going to use for bonding.
    #[clap(
        long,
        value_delimiter = ',',
        env = NYMNODE_PUBLIC_IPS_ARG
    )]
    pub(crate) public_ips: Option<Vec<IpAddr>>,

    /// Optional hostname associated with this gateway that will be announced to the nym-api and subsequently to the clients
    #[clap(
        long,
        env = NYMNODE_HOSTNAME_ARG
    )]
    pub(crate) hostname: Option<String>,

    /// Optional **physical** location of this node's server.
    /// Either full country name (e.g. 'Poland'), two-letter alpha2 (e.g. 'PL'),
    /// three-letter alpha3 (e.g. 'POL') or three-digit numeric-3 (e.g. '616') can be provided.
    #[clap(
        long,
        env = NYMNODE_LOCATION_ARG
    )]
    pub(crate) location: Option<Country>,
}

impl HostArgs {
    // TODO: could we perhaps make a clap error here and call `safe_exit` instead?
    pub(crate) fn build_config_section(self) -> config::Host {
        self.override_config_section(config::Host::default())
    }

    pub(crate) fn override_config_section(self, mut section: config::Host) -> config::Host {
        if let Some(public_ips) = self.public_ips {
            section.public_ips = public_ips
        }
        if let Some(hostname) = self.hostname {
            section.hostname = Some(hostname)
        }
        if let Some(location) = self.location {
            section.location = Some(location)
        }
        section
    }
}

#[derive(clap::Args, Debug)]
pub(crate) struct HttpArgs {
    /// Socket address this node will use for binding its http API.
    /// default: `0.0.0.0:8080`
    #[clap(
        long,
        env = NYMNODE_HTTP_BIND_ADDRESS_ARG
    )]
    pub(crate) http_bind_address: Option<SocketAddr>,

    /// Path to assets directory of custom landing page of this node.
    #[clap(
        long,
        env = NYMNODE_HTTP_LANDING_ASSETS_ARG
    )]
    pub(crate) landing_page_assets_path: Option<PathBuf>,

    /// An optional bearer token for accessing certain http endpoints.
    /// Currently only used for prometheus metrics.
    #[clap(
        long,
        env = NYMNODE_HTTP_ACCESS_TOKEN_ARG,
        alias = "http-bearer-token"
    )]
    pub(crate) http_access_token: Option<String>,

    /// Specify whether basic system information should be exposed.
    /// default: true
    #[clap(
        long,
        env = NYMNODE_HTTP_EXPOSE_SYSTEM_INFO_ARG,
    )]
    pub(crate) expose_system_info: Option<bool>,

    /// Specify whether basic system hardware information should be exposed.
    /// default: true
    #[clap(
        long,
        env = NYMNODE_HTTP_EXPOSE_SYSTEM_HARDWARE_ARG
    )]
    pub(crate) expose_system_hardware: Option<bool>,

    /// Specify whether detailed system crypto hardware information should be exposed.
    /// default: true
    #[clap(
        long,
        env = NYMNODE_HTTP_EXPOSE_CRYPTO_HARDWARE_ARG
    )]
    pub(crate) expose_crypto_hardware: Option<bool>,
}

impl HttpArgs {
    // TODO: could we perhaps make a clap error here and call `safe_exit` instead?
    pub(crate) fn build_config_section(self) -> config::Http {
        self.override_config_section(config::Http::default())
    }

    pub(crate) fn override_config_section(self, mut section: config::Http) -> config::Http {
        if let Some(bind_address) = self.http_bind_address {
            section.bind_address = bind_address
        }
        if let Some(landing_page_assets_path) = self.landing_page_assets_path {
            section.landing_page_assets_path = Some(landing_page_assets_path)
        }
        if let Some(access_token) = self.http_access_token {
            section.access_token = Some(access_token)
        }
        if let Some(expose_system_info) = self.expose_system_info {
            section.expose_system_info = expose_system_info
        }
        if let Some(expose_hardware_info) = self.expose_system_hardware {
            section.expose_system_hardware = expose_hardware_info
        }
        if let Some(expose_crypto_hardware) = self.expose_crypto_hardware {
            section.expose_crypto_hardware = expose_crypto_hardware
        }
        section
    }
}

#[derive(clap::Args, Debug)]
pub(crate) struct MixnetArgs {
    /// Address this node will bind to for listening for mixnet packets
    /// default: `0.0.0.0:1789`
    #[clap(
        long,
        env = NYMNODE_MIXNET_BIND_ADDRESS_ARG
    )]
    pub(crate) mixnet_bind_address: Option<SocketAddr>,

    /// Addresses to nym APIs from which the node gets the view of the network.
    #[clap(
        long,
        value_delimiter = ',',
        env = NYMNODE_NYM_APIS_ARG
    )]
    pub(crate) nym_api_urls: Option<Vec<Url>>,

    /// Addresses to nyxd chain endpoint which the node will use for chain interactions.
    #[clap(
        long,
        value_delimiter = ',',
        env = NYMNODE_NYXD_URLS_ARG
    )]
    pub(crate) nyxd_urls: Option<Vec<Url>>,

    /// Specifies whether this node should **NOT** use noise protocol in the connections (currently not implemented)
    #[clap(
        hide = true,
        long,
        env = NYMNODE_UNSAFE_DISABLE_NOISE
    )]
    pub(crate) unsafe_disable_noise: bool,
}

impl MixnetArgs {
    // TODO: could we perhaps make a clap error here and call `safe_exit` instead?
    pub(crate) fn build_config_section(self) -> config::Mixnet {
        self.override_config_section(config::Mixnet::default())
    }

    pub(crate) fn override_config_section(self, mut section: config::Mixnet) -> config::Mixnet {
        if let Some(bind_address) = self.mixnet_bind_address {
            section.bind_address = bind_address
        }
        if let Some(nym_api_urls) = self.nym_api_urls {
            section.nym_api_urls = nym_api_urls
        }
        if let Some(nyxd_urls) = self.nyxd_urls {
            section.nyxd_urls = nyxd_urls
        }
        if self.unsafe_disable_noise {
            section.debug.unsafe_disable_noise = true
        }
        section
    }
}

#[derive(clap::Args, Debug)]
pub(crate) struct WireguardArgs {
    /// Specifies whether the wireguard service is enabled on this node.
    #[clap(
        long,
        env = NYMNODE_WG_ENABLED_ARG
    )]
    pub(crate) wireguard_enabled: Option<bool>,

    /// Socket address this node will use for binding its wireguard interface.
    /// default: `0.0.0.0:51822`
    #[clap(
        long,
        env = NYMNODE_WG_BIND_ADDRESS_ARG
    )]
    pub(crate) wireguard_bind_address: Option<SocketAddr>,

    /// Private IP address of the wireguard gateway.
    /// default: `10.1.0.1`
    #[clap(
        long,
        env = NYMNODE_WG_IP_ARG,
    )]
    pub(crate) wireguard_private_ip: Option<IpAddr>,

    /// Port announced to external clients wishing to connect to the wireguard interface.
    /// Useful in the instances where the node is behind a proxy.
    #[clap(
        long,
        env = NYMNODE_WG_ANNOUNCED_PORT_ARG
    )]
    pub(crate) wireguard_announced_port: Option<u16>,

    /// The prefix denoting the maximum number of the clients that can be connected via Wireguard.
    /// The maximum value for IPv4 is 32 and for IPv6 is 128
    #[clap(
        long,
        env = NYMNODE_WG_PRIVATE_NETWORK_PREFIX_ARG
    )]
    pub(crate) wireguard_private_network_prefix: Option<u8>,
}

impl WireguardArgs {
    // TODO: could we perhaps make a clap error here and call `safe_exit` instead?
    pub(crate) fn build_config_section<P: AsRef<Path>>(self, data_dir: P) -> config::Wireguard {
        self.override_config_section(config::Wireguard::new_default(data_dir))
    }

    pub(crate) fn override_config_section(
        self,
        mut section: config::Wireguard,
    ) -> config::Wireguard {
        if let Some(enabled) = self.wireguard_enabled {
            section.enabled = enabled
        }

        if let Some(bind_address) = self.wireguard_bind_address {
            section.bind_address = bind_address
        }

        if let Some(announced_port) = self.wireguard_announced_port {
            section.announced_port = announced_port
        }

        if let Some(private_ip) = self.wireguard_private_ip {
            section.private_ip = private_ip
        }

        if let Some(private_network_prefix) = self.wireguard_private_network_prefix {
            section.private_network_prefix = private_network_prefix
        }

        section
    }
}

#[derive(clap::Args, Debug)]
pub(crate) struct MixnodeArgs {
    /// Socket address this node will use for binding its verloc API.
    /// default: `0.0.0.0:1790`
    #[clap(
        long,
        env = NYMNODE_VERLOC_BIND_ADDRESS_ARG
    )]
    pub(crate) verloc_bind_address: Option<SocketAddr>,
}

impl MixnodeArgs {
    // TODO: could we perhaps make a clap error here and call `safe_exit` instead?
    pub(crate) fn build_config_section(self) -> config::MixnodeConfig {
        self.override_config_section(config::MixnodeConfig::new_default())
    }

    pub(crate) fn override_config_section(
        self,
        mut section: config::MixnodeConfig,
    ) -> config::MixnodeConfig {
        if let Some(bind_address) = self.verloc_bind_address {
            section.verloc.bind_address = bind_address
        }
        section
    }
}

#[derive(clap::Args, Debug, Zeroize, ZeroizeOnDrop)]
pub(crate) struct EntryGatewayArgs {
    /// Socket address this node will use for binding its client websocket API.
    /// default: `0.0.0.0:9000`
    #[clap(
        long,
        env = NYMNODE_ENTRY_BIND_ADDRESS_ARG
    )]
    #[zeroize(skip)]
    pub(crate) entry_bind_address: Option<SocketAddr>,

    /// Custom announced port for listening for websocket client traffic.
    /// If unspecified, the value from the `bind_address` will be used instead
    #[clap(
        long,
        env = NYMNODE_ENTRY_ANNOUNCE_WS_PORT_ARG
    )]
    pub(crate) announce_ws_port: Option<u16>,

    /// If applicable, announced port for listening for secure websocket client traffic.
    #[clap(
        long,
        env = NYMNODE_ENTRY_ANNOUNCE_WSS_PORT_ARG
    )]
    pub(crate) announce_wss_port: Option<u16>,

    /// Indicates whether this gateway is accepting only coconut credentials for accessing the mixnet
    /// or if it also accepts non-paying clients
    #[clap(
        long,
        env = NYMNODE_ENFORCE_ZK_NYMS_ARG
    )]
    pub(crate) enforce_zk_nyms: Option<bool>,

    /// Indicates whether this gateway is using offline setup for zk-nyms verification
    #[clap(
        long,
        env = NYMNODE_OFFLINE_ZK_NYMS_ARG
    )]
    pub(crate) offline_zk_nyms: Option<bool>,

    /// Custom cosmos wallet mnemonic used for zk-nym redemption.
    /// If no value is provided, a fresh mnemonic is going to be generated.
    #[clap(
        long,
        env = NYMNODE_MNEMONIC_ARG
    )]
    pub(crate) mnemonic: Option<bip39::Mnemonic>,
}

impl EntryGatewayArgs {
    // TODO: could we perhaps make a clap error here and call `safe_exit` instead?
    pub(crate) fn build_config_section<P: AsRef<Path>>(
        self,
        data_dir: P,
    ) -> config::EntryGatewayConfig {
        self.override_config_section(config::EntryGatewayConfig::new_default(data_dir))
    }

    pub(crate) fn override_config_section(
        self,
        mut section: config::EntryGatewayConfig,
    ) -> config::EntryGatewayConfig {
        if let Some(bind_address) = self.entry_bind_address {
            section.bind_address = bind_address
        }
        if let Some(ws_port) = self.announce_ws_port {
            section.announce_ws_port = Some(ws_port)
        }
        if let Some(wss_port) = self.announce_wss_port {
            section.announce_wss_port = Some(wss_port)
        }
        if let Some(enforce_zk_nyms) = self.enforce_zk_nyms {
            section.enforce_zk_nyms = enforce_zk_nyms
        }

        if let Some(offline_zk_nyms) = self.offline_zk_nyms {
            section.offline_zk_nyms = offline_zk_nyms
        }

        section
    }
}

#[derive(clap::Args, Debug)]
pub(crate) struct ExitGatewayArgs {
    /// Specifies the url for an upstream source of the exit policy used by this node.
    #[clap(
        long,
        env = NYMNODE_UPSTREAM_EXIT_POLICY_ARG,
    )]
    pub(crate) upstream_exit_policy_url: Option<Url>,

    /// Specifies whether this exit node should run in 'open-proxy' mode
    /// and thus would attempt to resolve **ANY** request it receives.
    #[clap(
        long,
        env = NYMNODE_OPEN_PROXY_ARG,
    )]
    pub(crate) open_proxy: Option<bool>,
}

impl ExitGatewayArgs {
    // TODO: could we perhaps make a clap error here and call `safe_exit` instead?
    pub(crate) fn build_config_section<P: AsRef<Path>>(
        self,
        data_dir: P,
    ) -> config::ExitGatewayConfig {
        self.override_config_section(config::ExitGatewayConfig::new_default(data_dir))
    }

    pub(crate) fn override_config_section(
        self,
        mut section: config::ExitGatewayConfig,
    ) -> config::ExitGatewayConfig {
        if let Some(upstream_exit_policy) = self.upstream_exit_policy_url {
            section.upstream_exit_policy_url = upstream_exit_policy
        }
        if let Some(open_proxy) = self.open_proxy {
            section.open_proxy = open_proxy
        }

        section
    }
}

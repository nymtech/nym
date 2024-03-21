// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::helpers::ConfigArgs;
use crate::env::vars::*;
use nym_bin_common::output_format::OutputFormat;
use nym_node::config;
use nym_node::config::persistence::NymNodePaths;
use nym_node::config::{Config, ConfigBuilder, NodeMode};
use nym_node::error::NymNodeError;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use url::Url;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(flatten)]
    pub(crate) config: ConfigArgs,

    /// Forbid a new node from being initialised if configuration file for the provided specification doesn't already exist
    #[clap(
        long,
        default_value_t = false,
        env = NYMNODE_DENY_INIT_ARG,
        conflicts_with = "init_only"
    )]
    pub(crate) deny_init: bool,

    /// If this is a brand new nym-node, specify whether it should only be initialised without actually running the subprocesses.
    #[clap(
        long,
        default_value_t = false,
        env = NYMNODE_INIT_ONLY_ARG,
        conflicts_with = "deny_init"
    )]
    pub(crate) init_only: bool,

    /// Specifies the current mode of this nym-node.
    #[clap(
        long,
        value_enum,
        env = NYMNODE_MODE_ARG
    )]
    pub(crate) mode: Option<NodeMode>,

    /// If this node has been initialised before, specify whether to write any new changes to the config file.
    #[clap(
        short,
        long,
        default_value_t = false,
        env = NYMMONDE_WRITE_CONFIG_CHANGES_ARG,
    )]
    pub(crate) write_changes: bool,

    /// Specify output file for bonding information of this nym-node, i.e. its encoded keys.
    /// NOTE: the required bonding information is still a subject to change and this argument should be treated
    /// only as a preview of future features.
    #[clap(
        long,
        env = NYMNODE_BONDING_INFORMATION_OUTPUT_ARG
    )]
    pub(crate) bonding_information_output: Option<PathBuf>,

    /// Specify the output format of the bonding information (`text` or `json`)
    #[clap(
        short,
        long,
        default_value_t = OutputFormat::default(),
        env = NYMNODE_OUTPUT_ARG
    )]
    pub(crate) output: OutputFormat,

    #[clap(flatten)]
    host: HostArgs,

    #[clap(flatten)]
    http: HttpArgs,

    #[clap(flatten)]
    mixnet: MixnetArgs,

    #[clap(flatten)]
    wireguard: WireguardArgs,

    #[clap(flatten)]
    mixnode: MixnodeArgs,

    #[clap(flatten)]
    entry_gateway: EntryGatewayArgs,

    #[clap(flatten)]
    exit_gateway: ExitGatewayArgs,
}

impl Args {
    pub(super) fn take_mnemonic(&mut self) -> Option<Zeroizing<bip39::Mnemonic>> {
        self.entry_gateway.mnemonic.take().map(Zeroizing::new)
    }
}

#[derive(clap::Args, Debug)]
struct HostArgs {
    /// Comma separated list of public ip addresses that will be announced to the nym-api and subsequently to the clients.
    /// In nearly all circumstances, it's going to be identical to the address you're going to use for bonding.
    #[clap(
        long,
        value_delimiter = ',',
        env = NYMNODE_PUBLIC_IPS_ARG
    )]
    public_ips: Option<Vec<IpAddr>>,

    /// Optional hostname associated with this gateway that will be announced to the nym-api and subsequently to the clients
    #[clap(
        long,
        env = NYMNODE_HOSTNAME_ARG
    )]
    hostname: Option<String>,
}

impl HostArgs {
    // TODO: could we perhaps make a clap error here and call `safe_exit` instead?
    fn build_config_section(self) -> Result<config::Host, NymNodeError> {
        Ok(self.override_config_section(config::Host::default()))
    }

    fn override_config_section(self, mut section: config::Host) -> config::Host {
        if let Some(public_ips) = self.public_ips {
            section.public_ips = public_ips
        }
        if let Some(hostname) = self.hostname {
            section.hostname = Some(hostname)
        }
        section
    }
}

#[derive(clap::Args, Debug)]
struct HttpArgs {
    /// Socket address this node will use for binding its http API.
    /// default: `0.0.0.0:8080`
    #[clap(
        long,
        env = NYMNODE_HTTP_BIND_ADDRESS_ARG
    )]
    http_bind_address: Option<SocketAddr>,

    /// Path to assets directory of custom landing page of this node.
    #[clap(
        long,
        env = NYMNODE_HTTP_LANDING_ASSETS_ARG
    )]
    landing_page_assets_path: Option<PathBuf>,

    /// An optional bearer token for accessing certain http endpoints.
    /// Currently only used for obtaining mixnode's stats.
    #[clap(
        long,
        env = NYMNODE_HTTP_ACCESS_TOKEN_ARG
    )]
    http_access_token: Option<String>,
}

impl HttpArgs {
    // TODO: could we perhaps make a clap error here and call `safe_exit` instead?
    fn build_config_section(self) -> Result<config::Http, NymNodeError> {
        Ok(self.override_config_section(config::Http::default()))
    }

    fn override_config_section(self, mut section: config::Http) -> config::Http {
        if let Some(bind_address) = self.http_bind_address {
            section.bind_address = bind_address
        }
        if let Some(landing_page_assets_path) = self.landing_page_assets_path {
            section.landing_page_assets_path = Some(landing_page_assets_path)
        }
        if let Some(access_token) = self.http_access_token {
            section.access_token = Some(access_token)
        }
        section
    }
}

#[derive(clap::Args, Debug)]
struct MixnetArgs {
    /// Address this node will bind to for listening for mixnet packets
    /// default: `0.0.0.0:1789`
    #[clap(
        long,
        env = NYMNODE_MIXNET_BIND_ADDRESS_ARG
    )]
    mixnet_bind_address: Option<SocketAddr>,

    /// Addresses to nym APIs from which the node gets the view of the network.
    #[clap(
        long,
        value_delimiter = ',',
        env = NYMNODE_NYM_APIS_ARG
    )]
    nym_api_urls: Option<Vec<Url>>,
}

impl MixnetArgs {
    // TODO: could we perhaps make a clap error here and call `safe_exit` instead?
    fn build_config_section(self) -> Result<config::Mixnet, NymNodeError> {
        Ok(self.override_config_section(config::Mixnet::default()))
    }

    fn override_config_section(self, mut section: config::Mixnet) -> config::Mixnet {
        if let Some(bind_address) = self.mixnet_bind_address {
            section.bind_address = bind_address
        }
        if let Some(nym_api_urls) = self.nym_api_urls {
            section.nym_api_urls = nym_api_urls
        }
        section
    }
}

#[derive(clap::Args, Debug)]
struct WireguardArgs {
    /// Specifies whether the wireguard service is enabled on this node.
    #[clap(
        long,
        env = NYMNODE_WG_ENABLED_ARG
    )]
    wireguard_enabled: Option<bool>,

    /// Socket address this node will use for binding its wireguard interface.
    /// default: `0.0.0.0:51822`
    #[clap(
        long,
        env = NYMNODE_WG_BIND_ADDRESS_ARG
    )]
    wireguard_bind_address: Option<SocketAddr>,

    /// Port announced to external clients wishing to connect to the wireguard interface.
    /// Useful in the instances where the node is behind a proxy.
    #[clap(
        long,
        env = NYMNODE_WG_ANNOUNCED_PORT_ARG
    )]
    wireguard_announced_port: Option<u16>,

    /// The prefix denoting the maximum number of the clients that can be connected via Wireguard.
    /// The maximum value for IPv4 is 32 and for IPv6 is 128
    #[clap(
        long,
        env = NYMNODE_WG_PRIVATE_NETWORK_PREFIX_ARG
    )]
    wireguard_private_network_prefix: Option<u8>,
}

impl WireguardArgs {
    // TODO: could we perhaps make a clap error here and call `safe_exit` instead?
    fn build_config_section<P: AsRef<Path>>(
        self,
        data_dir: P,
    ) -> Result<config::Wireguard, NymNodeError> {
        Ok(self.override_config_section(config::Wireguard::new_default(data_dir)))
    }

    fn override_config_section(self, mut section: config::Wireguard) -> config::Wireguard {
        if let Some(enabled) = self.wireguard_enabled {
            section.enabled = enabled
        }

        if let Some(bind_address) = self.wireguard_bind_address {
            section.bind_address = bind_address
        }

        if let Some(announced_port) = self.wireguard_announced_port {
            section.announced_port = announced_port
        }

        if let Some(private_network_prefix) = self.wireguard_private_network_prefix {
            section.private_network_prefix = private_network_prefix
        }

        section
    }
}

#[derive(clap::Args, Debug)]
struct MixnodeArgs {
    /// Socket address this node will use for binding its verloc API.
    /// default: `0.0.0.0:1790`
    #[clap(
        long,
        env = NYMNODE_VERLOC_BIND_ADDRESS_ARG
    )]
    verloc_bind_address: Option<SocketAddr>,
}

impl MixnodeArgs {
    // TODO: could we perhaps make a clap error here and call `safe_exit` instead?
    fn build_config_section<P: AsRef<Path>>(
        self,
        config_dir: P,
    ) -> Result<config::MixnodeConfig, NymNodeError> {
        Ok(self.override_config_section(config::MixnodeConfig::new_default(config_dir)))
    }

    fn override_config_section(self, mut section: config::MixnodeConfig) -> config::MixnodeConfig {
        if let Some(bind_address) = self.verloc_bind_address {
            section.verloc.bind_address = bind_address
        }
        section
    }
}

#[derive(clap::Args, Debug, Zeroize, ZeroizeOnDrop)]
struct EntryGatewayArgs {
    /// Socket address this node will use for binding its client websocket API.
    /// default: `0.0.0.0:9000`
    #[clap(
        long,
        env = NYMNODE_ENTRY_BIND_ADDRESS_ARG
    )]
    #[zeroize(skip)]
    entry_bind_address: Option<SocketAddr>,

    /// Custom announced port for listening for websocket client traffic.
    /// If unspecified, the value from the `bind_address` will be used instead
    #[clap(
        long,
        env = NYMNODE_ENTRY_ANNOUNCE_WS_PORT_ARG
    )]
    announce_ws_port: Option<u16>,

    /// If applicable, announced port for listening for secure websocket client traffic.
    #[clap(
        long,
        env = NYMNODE_ENTRY_ANNOUNCE_WSS_PORT_ARG
    )]
    announce_wss_port: Option<u16>,

    /// Indicates whether this gateway is accepting only coconut credentials for accessing the mixnet
    /// or if it also accepts non-paying clients
    #[clap(
        long,
        env = NYMNODE_ENFORCE_ZK_NYMS_ARG
    )]
    enforce_zk_nyms: Option<bool>,

    /// Custom cosmos wallet mnemonic used for zk-nym redemption.
    /// If no value is provided, a fresh mnemonic is going to be generated.
    #[clap(
        long,
        env = NYMNODE_MNEMONIC_ARG
    )]
    mnemonic: Option<bip39::Mnemonic>,
}

impl EntryGatewayArgs {
    // TODO: could we perhaps make a clap error here and call `safe_exit` instead?
    fn build_config_section<P: AsRef<Path>>(
        self,
        data_dir: P,
    ) -> Result<config::EntryGatewayConfig, NymNodeError> {
        Ok(self.override_config_section(config::EntryGatewayConfig::new_default(data_dir)))
    }

    fn override_config_section(
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

        section
    }
}

#[derive(clap::Args, Debug)]
struct ExitGatewayArgs {
    /// Specifies the url for an upstream source of the exit policy used by this node.
    #[clap(
        long,
        env = NYMNODE_UPSTREAM_EXIT_POLICY_ARG,
    )]
    upstream_exit_policy_url: Option<Url>,

    /// Specifies whether this exit node should run in 'open-proxy' mode
    /// and thus would attempt to resolve **ANY** request it receives.
    #[clap(
        long,
        env = NYMNODE_OPEN_PROXY_ARG,
    )]
    open_proxy: Option<bool>,
}

impl ExitGatewayArgs {
    // TODO: could we perhaps make a clap error here and call `safe_exit` instead?
    fn build_config_section<P: AsRef<Path>>(
        self,
        data_dir: P,
    ) -> Result<config::ExitGatewayConfig, NymNodeError> {
        Ok(self.override_config_section(config::ExitGatewayConfig::new_default(data_dir)))
    }

    fn override_config_section(
        self,
        mut section: config::ExitGatewayConfig,
    ) -> config::ExitGatewayConfig {
        if let Some(upstream_exit_policy) = self.upstream_exit_policy_url {
            section.upstream_exit_policy_url = Some(upstream_exit_policy)
        }
        if let Some(open_proxy) = self.open_proxy {
            section.open_proxy = open_proxy
        }

        section
    }
}

impl Args {
    pub(crate) fn build_config(self) -> Result<Config, NymNodeError> {
        let config_path = self.config.config_path();
        let data_dir = Config::default_data_directory(&config_path)?;
        let config_dir = config_path
            .parent()
            .ok_or(NymNodeError::ConfigDirDerivationFailure)?;

        let id = self
            .config
            .id()
            .clone()
            .ok_or(NymNodeError::MissingInitArg {
                section: "global".to_string(),
                name: "id".to_string(),
            })?;

        ConfigBuilder::new(id, config_path.clone(), data_dir.clone())
            .with_mode(self.mode.unwrap_or_default())
            .with_host(self.host.build_config_section()?)
            .with_http(self.http.build_config_section()?)
            .with_mixnet(self.mixnet.build_config_section()?)
            .with_wireguard(self.wireguard.build_config_section(&data_dir)?)
            .with_storage_paths(NymNodePaths::new(&data_dir))
            .with_mixnode(self.mixnode.build_config_section(config_dir)?)
            .with_entry_gateway(self.entry_gateway.build_config_section(&data_dir)?)
            .with_exit_gateway(self.exit_gateway.build_config_section(&data_dir)?)
            .build()
    }

    pub(crate) fn override_config(self, mut config: Config) -> Config {
        if let Some(mode) = self.mode {
            config.mode = mode;
        }
        config.host = self.host.override_config_section(config.host);
        config.http = self.http.override_config_section(config.http);
        config.mixnet = self.mixnet.override_config_section(config.mixnet);
        config.wireguard = self.wireguard.override_config_section(config.wireguard);
        config.mixnode = self.mixnode.override_config_section(config.mixnode);
        config.entry_gateway = self
            .entry_gateway
            .override_config_section(config.entry_gateway);
        config.exit_gateway = self
            .exit_gateway
            .override_config_section(config.exit_gateway);
        config
    }
}

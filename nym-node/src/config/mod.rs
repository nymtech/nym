// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::persistence::{NymNodePaths, ReplayProtectionPaths};
use crate::config::template::CONFIG_TEMPLATE;
use crate::error::NymNodeError;
use celes::Country;
use clap::ValueEnum;
use human_repr::HumanCount;
use nym_bin_common::logging::LoggingSettings;
use nym_config::defaults::{
    mainnet, var_names, DEFAULT_MIX_LISTENING_PORT, DEFAULT_NYM_NODE_HTTP_PORT,
    DEFAULT_VERLOC_LISTENING_PORT, WG_PORT, WG_TUN_DEVICE_IP_ADDRESS_V4,
    WG_TUN_DEVICE_IP_ADDRESS_V6,
};
use nym_config::defaults::{WG_TUN_DEVICE_NETMASK_V4, WG_TUN_DEVICE_NETMASK_V6};
use nym_config::helpers::{in6addr_any_init, inaddr_any};
use nym_config::serde_helpers::de_maybe_port;
use nym_config::serde_helpers::de_maybe_stringified;
use nym_config::{
    must_get_home, parse_urls, read_config_from_toml_file, save_formatted_config_to_file,
    NymConfigTemplate, DEFAULT_CONFIG_DIR, DEFAULT_CONFIG_FILENAME, DEFAULT_DATA_DIR, NYM_DIR,
};
use nym_gateway::nym_authenticator;
use serde::{Deserialize, Serialize};
use std::env;
use std::fmt::{Display, Formatter};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::time::Duration;
use sysinfo::System;
use tracing::{debug, error};
use url::Url;

pub mod authenticator;
pub mod gateway_tasks;
pub mod helpers;
pub mod metrics;
mod old_configs;
pub mod persistence;
pub mod service_providers;
mod template;
pub mod upgrade_helpers;

pub use crate::config::gateway_tasks::GatewayTasksConfig;
pub use crate::config::metrics::MetricsConfig;
pub use crate::config::service_providers::ServiceProvidersConfig;
use crate::node::replay_protection::{bitmap_size, items_in_bloomfilter};

const DEFAULT_NYMNODES_DIR: &str = "nym-nodes";

pub const DEFAULT_HTTP_PORT: u16 = DEFAULT_NYM_NODE_HTTP_PORT;
pub const DEFAULT_MIXNET_PORT: u16 = DEFAULT_MIX_LISTENING_PORT;

/// Derive default path to nym-node's config directory.
/// It should get resolved to `$HOME/.nym/nym-nodes/<id>/config`
pub fn default_config_directory<P: AsRef<Path>>(id: P) -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_NYMNODES_DIR)
        .join(id)
        .join(DEFAULT_CONFIG_DIR)
}

/// Derive default path to nym-node's config file.
/// It should get resolved to `$HOME/.nym/nym-nodes/<id>/config/config.toml`
pub fn default_config_filepath<P: AsRef<Path>>(id: P) -> PathBuf {
    default_config_directory(id).join(DEFAULT_CONFIG_FILENAME)
}

// a temporary solution until all "types" are run at the same time
#[derive(Debug, Default, Serialize, Deserialize, ValueEnum, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum NodeMode {
    #[default]
    #[clap(alias = "mix")]
    Mixnode,

    #[clap(alias = "entry", alias = "gateway")]
    EntryGateway,

    // to not break existing behaviour, this means exit capabilities AND entry capabilities
    #[clap(alias = "exit")]
    ExitGateway,

    // will start only SP needed for exit capabilities WITHOUT entry routing
    ExitProvidersOnly,
}

impl Display for NodeMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeMode::Mixnode => "mixnode".fmt(f),
            NodeMode::EntryGateway => "entry-gateway".fmt(f),
            NodeMode::ExitGateway => "exit-gateway".fmt(f),
            NodeMode::ExitProvidersOnly => "exit-providers-only".fmt(f),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy)]
pub struct NodeModes {
    /// Specifies whether this node can operate in a mixnode mode.
    pub mixnode: bool,

    /// Specifies whether this node can operate in an entry mode.
    pub entry: bool,

    /// Specifies whether this node can operate in an exit mode.
    pub exit: bool,
    // TODO: would it make sense to also put WG here for completion?
}

impl From<&[NodeMode]> for NodeModes {
    fn from(modes: &[NodeMode]) -> Self {
        let mut out = NodeModes::default();
        for &mode in modes {
            out.with_mode(mode);
        }
        out
    }
}

impl NodeModes {
    pub fn any_enabled(&self) -> bool {
        self.mixnode || self.entry || self.exit
    }

    pub fn standalone_exit(&self) -> bool {
        !self.mixnode && !self.entry && self.exit
    }

    pub fn with_mode(&mut self, mode: NodeMode) -> &mut Self {
        match mode {
            NodeMode::Mixnode => self.with_mixnode(),
            NodeMode::EntryGateway => self.with_entry(),
            NodeMode::ExitGateway => self.with_entry().with_exit(),
            NodeMode::ExitProvidersOnly => self.with_exit(),
        }
    }

    pub fn expects_final_hop_traffic(&self) -> bool {
        self.entry || self.exit
    }

    pub fn with_mixnode(&mut self) -> &mut Self {
        self.mixnode = true;
        self
    }

    pub fn with_entry(&mut self) -> &mut Self {
        self.entry = true;
        self
    }

    pub fn with_exit(&mut self) -> &mut Self {
        self.exit = true;
        self
    }
}

pub struct ConfigBuilder {
    pub id: String,

    pub config_path: PathBuf,

    pub data_dir: PathBuf,

    pub modes: NodeModes,

    pub mixnet: Option<Mixnet>,

    pub host: Option<Host>,

    pub http: Option<Http>,

    pub verloc: Option<Verloc>,

    pub wireguard: Option<Wireguard>,

    pub storage_paths: Option<NymNodePaths>,

    pub gateway_tasks: Option<GatewayTasksConfig>,

    pub service_providers: Option<ServiceProvidersConfig>,

    pub metrics: Option<MetricsConfig>,

    pub logging: Option<LoggingSettings>,
}

impl ConfigBuilder {
    pub fn new(id: String, config_path: PathBuf, data_dir: PathBuf) -> Self {
        ConfigBuilder {
            id,
            config_path,
            data_dir,
            host: None,
            http: None,
            mixnet: None,
            verloc: None,
            wireguard: None,
            modes: NodeModes::default(),
            storage_paths: None,
            gateway_tasks: None,
            service_providers: None,
            metrics: None,
            logging: None,
        }
    }

    pub fn with_modes(mut self, mode: impl Into<NodeModes>) -> Self {
        self.modes = mode.into();
        self
    }

    pub fn with_host(mut self, section: impl Into<Option<Host>>) -> Self {
        self.host = section.into();
        self
    }

    pub fn with_http(mut self, section: impl Into<Option<Http>>) -> Self {
        self.http = section.into();
        self
    }

    pub fn with_verloc(mut self, section: impl Into<Option<Verloc>>) -> Self {
        self.verloc = section.into();
        self
    }

    pub fn with_mixnet(mut self, section: impl Into<Option<Mixnet>>) -> Self {
        self.mixnet = section.into();
        self
    }

    pub fn with_wireguard(mut self, section: impl Into<Option<Wireguard>>) -> Self {
        self.wireguard = section.into();
        self
    }

    pub fn with_storage_paths(mut self, section: impl Into<Option<NymNodePaths>>) -> Self {
        self.storage_paths = section.into();
        self
    }

    pub fn with_metrics(mut self, section: impl Into<Option<MetricsConfig>>) -> Self {
        self.metrics = section.into();
        self
    }

    pub fn with_gateway_tasks(mut self, section: impl Into<Option<GatewayTasksConfig>>) -> Self {
        self.gateway_tasks = section.into();
        self
    }

    pub fn with_service_providers(
        mut self,
        section: impl Into<Option<ServiceProvidersConfig>>,
    ) -> Self {
        self.service_providers = section.into();
        self
    }

    pub fn build(self) -> Config {
        Config {
            id: self.id,
            modes: self.modes,
            host: self.host.unwrap_or_default(),
            http: self.http.unwrap_or_default(),
            mixnet: self
                .mixnet
                .unwrap_or_else(|| Mixnet::new_default(&self.data_dir)),
            verloc: self.verloc.unwrap_or_default(),
            wireguard: self
                .wireguard
                .unwrap_or_else(|| Wireguard::new_default(&self.data_dir)),
            storage_paths: self
                .storage_paths
                .unwrap_or_else(|| NymNodePaths::new(&self.data_dir)),
            metrics: self.metrics.unwrap_or_default(),
            gateway_tasks: self
                .gateway_tasks
                .unwrap_or_else(|| GatewayTasksConfig::new_default(&self.data_dir)),
            service_providers: self
                .service_providers
                .unwrap_or_else(|| ServiceProvidersConfig::new_default(&self.data_dir)),
            logging: self.logging.unwrap_or_default(),
            save_path: Some(self.config_path),
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    // additional metadata holding on-disk location of this config file
    #[serde(skip)]
    pub(crate) save_path: Option<PathBuf>,

    /// Human-readable ID of this particular node.
    pub id: String,

    /// Current modes of this nym-node.
    pub modes: NodeModes,

    pub host: Host,

    pub mixnet: Mixnet,

    /// Storage paths to persistent nym-node data, such as its long term keys.
    pub storage_paths: NymNodePaths,

    #[serde(default)]
    pub http: Http,

    #[serde(default)]
    pub verloc: Verloc,

    pub wireguard: Wireguard,

    #[serde(alias = "entry_gateway")]
    pub gateway_tasks: GatewayTasksConfig,

    #[serde(alias = "exit_gateway")]
    pub service_providers: ServiceProvidersConfig,

    #[serde(default)]
    pub metrics: MetricsConfig,

    #[serde(default)]
    pub logging: LoggingSettings,

    #[serde(default)]
    pub debug: Debug,
}

impl NymConfigTemplate for Config {
    fn template(&self) -> &'static str {
        CONFIG_TEMPLATE
    }
}

impl Config {
    pub fn save(&self) -> Result<(), NymNodeError> {
        let save_location = self.save_location();
        debug!(
            "attempting to save config file to '{}'",
            save_location.display()
        );
        save_formatted_config_to_file(self, &save_location).map_err(|source| {
            NymNodeError::ConfigSaveFailure {
                id: self.id.clone(),
                path: save_location,
                source,
            }
        })
    }

    pub fn save_location(&self) -> PathBuf {
        self.save_path
            .clone()
            .unwrap_or(self.default_save_location())
    }

    pub fn default_save_location(&self) -> PathBuf {
        default_config_filepath(&self.id)
    }

    pub fn default_data_directory<P: AsRef<Path>>(config_path: P) -> Result<PathBuf, NymNodeError> {
        let config_path = config_path.as_ref();

        // we got a proper path to the .toml file
        let Some(config_dir) = config_path.parent() else {
            error!(
                "'{}' does not have a parent directory. Have you pointed to the fs root?",
                config_path.display()
            );
            return Err(NymNodeError::DataDirDerivationFailure);
        };

        let Some(config_dir_name) = config_dir.file_name() else {
            error!(
                "could not obtain parent directory name of '{}'. Have you used relative paths?",
                config_path.display()
            );
            return Err(NymNodeError::DataDirDerivationFailure);
        };

        if config_dir_name != DEFAULT_CONFIG_DIR {
            error!(
                "the parent directory of '{}' ({}) is not {DEFAULT_CONFIG_DIR}. currently this is not supported",
                config_path.display(), config_dir_name.to_str().unwrap_or("UNKNOWN")
            );
            return Err(NymNodeError::DataDirDerivationFailure);
        }

        let Some(node_dir) = config_dir.parent() else {
            error!(
                "'{}' does not have a parent directory. Have you pointed to the fs root?",
                config_dir.display()
            );
            return Err(NymNodeError::DataDirDerivationFailure);
        };

        Ok(node_dir.join(DEFAULT_DATA_DIR))
    }

    // simple wrapper that reads config file and assigns path location
    fn read_from_path<P: AsRef<Path>>(path: P) -> Result<Self, NymNodeError> {
        let path = path.as_ref();
        let mut loaded: Config =
            read_config_from_toml_file(path).map_err(|source| NymNodeError::ConfigLoadFailure {
                path: path.to_path_buf(),
                source,
            })?;
        loaded.save_path = Some(path.to_path_buf());
        debug!("loaded config file from {}", path.display());
        Ok(loaded)
    }

    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> Result<Self, NymNodeError> {
        Self::read_from_path(path)
    }

    pub fn validate(&self) -> Result<(), NymNodeError> {
        self.mixnet.validate()?;

        // it's not allowed to run mixnode mode alongside entry mode
        if self.modes.mixnode && self.modes.entry {
            return Err(NymNodeError::config_validation_failure(
                "illegal modes configuration - node cannot run as a mixnode and an entry gateway",
            ));
        }

        // nor it's allowed to run mixnode mode alongside exit mode
        // (use two separate checks for better error messages)
        if self.modes.mixnode && self.modes.exit {
            return Err(NymNodeError::config_validation_failure(
                "illegal modes configuration - node cannot run as a mixnode and an exit gateway",
            ));
        }

        Ok(())
    }
}

// TODO: this is very much a WIP. we need proper ssl certificate support here
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct Host {
    /// Ip address(es) of this host, such as 1.1.1.1 that external clients will use for connections.
    /// If no values are provided, when this node gets included in the network,
    /// its ip addresses will be populated by whatever value is resolved by associated nym-api.
    pub public_ips: Vec<IpAddr>,

    /// Optional hostname of this node, for example nymtech.net.
    // TODO: this is temporary. to be replaced by pulling the data directly from the certs.
    #[serde(deserialize_with = "de_maybe_stringified")]
    pub hostname: Option<String>,

    /// Optional ISO 3166 alpha-2 two-letter country code of the node's **physical** location
    #[serde(deserialize_with = "de_maybe_stringified")]
    pub location: Option<Country>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct Http {
    /// Socket address this node will use for binding its http API.
    /// default: `[::]:8080`
    pub bind_address: SocketAddr,

    /// Path to assets directory of custom landing page of this node.
    #[serde(deserialize_with = "de_maybe_stringified")]
    pub landing_page_assets_path: Option<PathBuf>,

    /// An optional bearer token for accessing certain http endpoints.
    /// Currently only used for obtaining mixnode's stats.
    #[serde(default)]
    #[serde(deserialize_with = "de_maybe_stringified")]
    pub access_token: Option<String>,

    /// Specify whether basic system information should be exposed.
    /// default: true
    pub expose_system_info: bool,

    /// Specify whether basic system hardware information should be exposed.
    /// This option is superseded by `expose_system_info`
    /// default: true
    pub expose_system_hardware: bool,

    /// Specify whether detailed system crypto hardware information should be exposed.
    /// This option is superseded by `expose_system_hardware`
    /// default: true
    pub expose_crypto_hardware: bool,

    /// Specify the cache ttl of the node load.
    /// default: 30s
    #[serde(with = "humantime_serde")]
    pub node_load_cache_ttl: Duration,
}

impl Http {
    pub const DEFAULT_NODE_LOAD_CACHE_TTL: Duration = Duration::from_secs(30);
}

impl Default for Http {
    fn default() -> Self {
        Http {
            bind_address: SocketAddr::new(in6addr_any_init(), DEFAULT_HTTP_PORT),
            landing_page_assets_path: None,
            access_token: None,
            expose_system_info: true,
            expose_system_hardware: true,
            expose_crypto_hardware: true,
            node_load_cache_ttl: Self::DEFAULT_NODE_LOAD_CACHE_TTL,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Mixnet {
    /// Address this node will bind to for listening for mixnet packets
    /// default: `[::]:1789`
    pub bind_address: SocketAddr,

    /// If applicable, custom port announced in the self-described API that other clients and nodes
    /// will use.
    /// Useful when the node is behind a proxy.
    #[serde(deserialize_with = "de_maybe_port")]
    #[serde(default)]
    pub announce_port: Option<u16>,

    /// Addresses to nym APIs from which the node gets the view of the network.
    pub nym_api_urls: Vec<Url>,

    /// Addresses to nyxd which the node uses to interact with the nyx chain.
    pub nyxd_urls: Vec<Url>,

    /// Settings for controlling replay detection
    pub replay_protection: ReplayProtection,

    #[serde(default)]
    pub key_rotation: KeyRotation,

    #[serde(default)]
    pub debug: MixnetDebug,
}

impl Mixnet {
    pub fn validate(&self) -> Result<(), NymNodeError> {
        if self.nym_api_urls.is_empty() {
            return Err(NymNodeError::config_validation_failure(
                "no nym api urls provided",
            ));
        }

        if self.nyxd_urls.is_empty() {
            return Err(NymNodeError::config_validation_failure(
                "no nyxd urls provided",
            ));
        }

        self.replay_protection.validate()?;

        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct MixnetDebug {
    /// Specifies the duration of time this node is willing to delay a forward packet for.
    #[serde(with = "humantime_serde")]
    pub maximum_forward_packet_delay: Duration,

    /// Initial value of an exponential backoff to reconnect to dropped TCP connection when
    /// forwarding sphinx packets.
    #[serde(with = "humantime_serde")]
    pub packet_forwarding_initial_backoff: Duration,

    /// Maximum value of an exponential backoff to reconnect to dropped TCP connection when
    /// forwarding sphinx packets.
    #[serde(with = "humantime_serde")]
    pub packet_forwarding_maximum_backoff: Duration,

    /// Timeout for establishing initial connection when trying to forward a sphinx packet.
    #[serde(with = "humantime_serde")]
    pub initial_connection_timeout: Duration,

    /// Maximum number of packets that can be stored waiting to get sent to a particular connection.
    pub maximum_connection_buffer_size: usize,

    /// Specify whether any framed packets between nodes should use the legacy format (v7)
    /// as opposed to the current (v8) one.
    /// The legacy format has to be used until sufficient number of nodes on the network has upgraded and understand the new variant.
    /// This will allow for optimisations to indicate which [sphinx] key is meant to be used when
    /// processing received packets.
    pub use_legacy_packet_encoding: bool,

    /// Specifies whether this node should **NOT** use noise protocol in the connections (currently not implemented)
    pub unsafe_disable_noise: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct ReplayProtection {
    /// Paths for current bloomfilters
    pub storage_paths: persistence::ReplayProtectionPaths,

    #[serde(default)]
    pub debug: ReplayProtectionDebug,
}

impl ReplayProtection {
    pub fn new_default<P: AsRef<Path>>(data_dir: P) -> Self {
        ReplayProtection {
            storage_paths: ReplayProtectionPaths::new(data_dir),
            debug: Default::default(),
        }
    }
}

impl ReplayProtection {
    pub fn validate(&self) -> Result<(), NymNodeError> {
        self.debug.validate()?;

        Ok(())
    }
}

#[derive(Debug, Copy, Clone, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct ReplayProtectionDebug {
    /// Specifies whether this node should **NOT** use replay protection
    pub unsafe_disabled: bool,

    /// How long the processing task is willing to skip mutex acquisition before it will block the thread
    /// until it actually obtains it
    pub maximum_replay_detection_deferral: Duration,

    /// How many packets the processing task is willing to queue before it will block the thread
    /// until it obtains the mutex
    pub maximum_replay_detection_pending_packets: usize,

    /// Probability of false positives, fraction between 0 and 1 or a number indicating 1-in-p
    pub false_positive_rate: f64,

    /// Defines initial expected number of packets this node will process a second,
    /// so that an initial bloomfilter could be established.
    /// As the node is running and BF are cleared, the value will be adjusted dynamically
    pub initial_expected_packets_per_second: usize,

    /// Defines minimum expected number of packets this node will process a second
    /// when used for calculating the BF size after reset.
    /// This is to avoid degenerate cases where node receives 0 packets (because say it's misconfigured)
    /// and it constructs an empty bloomfilter.
    pub bloomfilter_minimum_packets_per_second_size: usize,

    /// Specifies the amount the bloomfilter size is going to get multiplied by after each reset.
    /// It's performed in case the traffic rates increase before the next bloomfilter update.
    pub bloomfilter_size_multiplier: f64,

    /// Specifies how often the bloomfilter is flushed to disk for recovery in case of a crash
    #[serde(with = "humantime_serde")]
    pub bloomfilter_disk_flushing_rate: Duration,
}

impl ReplayProtectionDebug {
    pub const DEFAULT_MAXIMUM_REPLAY_DETECTION_DEFERRAL: Duration = Duration::from_millis(50);

    pub const DEFAULT_MAXIMUM_REPLAY_DETECTION_PENDING_PACKETS: usize = 100;

    // 12% (completely arbitrary)
    pub const DEFAULT_BLOOMFILTER_SIZE_MULTIPLIER: f64 = 1.12;

    // 10^-5
    pub const DEFAULT_REPLAY_DETECTION_FALSE_POSITIVE_RATE: f64 = 1e-5;

    // we must have some reasonable balance between losing values and trashing the disk.
    // since on average HDD it would take ~30s to save a 2GB bloomfilter
    pub const DEFAULT_BF_DISK_FLUSHING_RATE: Duration = Duration::from_secs(10 * 60);

    // this value will have to be adjusted in the future
    pub const DEFAULT_INITIAL_EXPECTED_PACKETS_PER_SECOND: usize = 2000;

    pub const DEFAULT_BLOOMFILTER_MINIMUM_PACKETS_PER_SECOND_SIZE: usize = 200;

    pub fn validate(&self) -> Result<(), NymNodeError> {
        if self.false_positive_rate >= 1.0 || self.false_positive_rate <= 0.0 {
            return Err(NymNodeError::config_validation_failure(
                "false positive rate for replay detection can't be larger than (or equal to) 1 or smaller than (or equal to) 0",
            ));
        }

        // ideally we would have pulled the exact information from the network,
        // but making async calls really doesn't play around with this method
        // so we do second best: assume 24h rotation with 1h overlap (which realistically won't ever change)

        let items_in_filter = items_in_bloomfilter(
            Duration::from_secs(25 * 60 * 60),
            self.initial_expected_packets_per_second,
        );
        let bitmap_size = bitmap_size(self.false_positive_rate, items_in_filter);
        let bloomfilter_size = bitmap_size / 8;

        let mut sys_info = System::new();
        sys_info.refresh_memory();

        // we'll need 2x size of the bloomfilter
        // as during key transition we'll have to simultaneously use two filters
        // plus we also need to make a memcopy during disk flush
        let required_memory = 2 * bloomfilter_size;

        let memory = sys_info.available_memory();
        if (memory as usize) < required_memory {
            return Err(NymNodeError::config_validation_failure(
                 format!("system does not have sufficient memory to allocate required replay protection bloomfilters. {} is available whilst at least {} is needed",memory.human_count_bytes(), required_memory.human_count_bytes())));
        }

        Ok(())
    }
}

impl Default for ReplayProtectionDebug {
    fn default() -> Self {
        ReplayProtectionDebug {
            unsafe_disabled: false,
            maximum_replay_detection_deferral: Self::DEFAULT_MAXIMUM_REPLAY_DETECTION_DEFERRAL,
            maximum_replay_detection_pending_packets:
                Self::DEFAULT_MAXIMUM_REPLAY_DETECTION_PENDING_PACKETS,
            false_positive_rate: Self::DEFAULT_REPLAY_DETECTION_FALSE_POSITIVE_RATE,
            initial_expected_packets_per_second: Self::DEFAULT_INITIAL_EXPECTED_PACKETS_PER_SECOND,
            bloomfilter_minimum_packets_per_second_size:
                Self::DEFAULT_BLOOMFILTER_MINIMUM_PACKETS_PER_SECOND_SIZE,
            bloomfilter_size_multiplier: Self::DEFAULT_BLOOMFILTER_SIZE_MULTIPLIER,
            bloomfilter_disk_flushing_rate: Self::DEFAULT_BF_DISK_FLUSHING_RATE,
        }
    }
}

#[derive(Debug, Default, Copy, Clone, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct KeyRotation {
    pub debug: KeyRotationDebug,
}

#[derive(Debug, Copy, Clone, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct KeyRotationDebug {
    /// Specifies how often the node should poll for any changes in the key rotation global state.
    #[serde(with = "humantime_serde")]
    pub rotation_state_poling_interval: Duration,
}

impl KeyRotationDebug {
    pub const DEFAULT_ROTATION_STATE_POLLING_INTERVAL: Duration = Duration::from_secs(4 * 60 * 60);
}

impl Default for KeyRotationDebug {
    fn default() -> Self {
        KeyRotationDebug {
            rotation_state_poling_interval: Self::DEFAULT_ROTATION_STATE_POLLING_INTERVAL,
        }
    }
}

impl MixnetDebug {
    // given that genuine clients are using mean delay of 50ms,
    // the probability of them delaying for over 10s is 10^-87
    // which for all intents and purposes will never happen
    const DEFAULT_MAXIMUM_FORWARD_PACKET_DELAY: Duration = Duration::from_secs(10);
    const DEFAULT_PACKET_FORWARDING_INITIAL_BACKOFF: Duration = Duration::from_millis(10_000);
    const DEFAULT_PACKET_FORWARDING_MAXIMUM_BACKOFF: Duration = Duration::from_millis(300_000);
    const DEFAULT_INITIAL_CONNECTION_TIMEOUT: Duration = Duration::from_millis(1_500);
    const DEFAULT_MAXIMUM_CONNECTION_BUFFER_SIZE: usize = 2000;
}

impl Default for MixnetDebug {
    fn default() -> Self {
        MixnetDebug {
            maximum_forward_packet_delay: Self::DEFAULT_MAXIMUM_FORWARD_PACKET_DELAY,
            packet_forwarding_initial_backoff: Self::DEFAULT_PACKET_FORWARDING_INITIAL_BACKOFF,
            packet_forwarding_maximum_backoff: Self::DEFAULT_PACKET_FORWARDING_MAXIMUM_BACKOFF,
            initial_connection_timeout: Self::DEFAULT_INITIAL_CONNECTION_TIMEOUT,
            maximum_connection_buffer_size: Self::DEFAULT_MAXIMUM_CONNECTION_BUFFER_SIZE,
            // TODO: update this in few releases...
            use_legacy_packet_encoding: true,
            unsafe_disable_noise: false,
        }
    }
}

impl Mixnet {
    pub fn new_default<P: AsRef<Path>>(data_dir: P) -> Self {
        // SAFETY:
        // our hardcoded values should always be valid
        #[allow(clippy::expect_used)]
        // is if there's anything set in the environment, otherwise fallback to mainnet
        let nym_api_urls = if let Ok(env_value) = env::var(var_names::NYM_API) {
            parse_urls(&env_value)
        } else {
            vec![mainnet::NYM_API.parse().expect("Invalid default API URL")]
        };

        #[allow(clippy::expect_used)]
        let nyxd_urls = if let Ok(env_value) = env::var(var_names::NYXD) {
            parse_urls(&env_value)
        } else {
            vec![mainnet::NYXD_URL.parse().expect("Invalid default nyxd URL")]
        };

        Mixnet {
            bind_address: SocketAddr::new(in6addr_any_init(), DEFAULT_MIXNET_PORT),
            announce_port: None,
            nym_api_urls,
            nyxd_urls,
            replay_protection: ReplayProtection::new_default(data_dir),
            key_rotation: Default::default(),
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Verloc {
    /// Socket address this node will use for binding its verloc API.
    /// default: `[::]:1790`
    pub bind_address: SocketAddr,

    /// If applicable, custom port announced in the self-described API that other clients and nodes
    /// will use.
    /// Useful when the node is behind a proxy.
    #[serde(deserialize_with = "de_maybe_port")]
    #[serde(default)]
    pub announce_port: Option<u16>,

    #[serde(default)]
    pub debug: VerlocDebug,
}

impl Verloc {
    pub const DEFAULT_VERLOC_PORT: u16 = DEFAULT_VERLOC_LISTENING_PORT;
}

impl Default for Verloc {
    fn default() -> Self {
        Verloc {
            bind_address: SocketAddr::new(in6addr_any_init(), Self::DEFAULT_VERLOC_PORT),
            announce_port: None,
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VerlocDebug {
    /// Specifies number of echo packets sent to each node during a measurement run.
    pub packets_per_node: usize,

    /// Specifies maximum amount of time to wait for the connection to get established.
    #[serde(with = "humantime_serde")]
    pub connection_timeout: Duration,

    /// Specifies maximum amount of time to wait for the reply packet to arrive before abandoning the test.
    #[serde(with = "humantime_serde")]
    pub packet_timeout: Duration,

    /// Specifies delay between subsequent test packets being sent (after receiving a reply).
    #[serde(with = "humantime_serde")]
    pub delay_between_packets: Duration,

    /// Specifies number of nodes being tested at once.
    pub tested_nodes_batch_size: usize,

    /// Specifies delay between subsequent test runs.
    #[serde(with = "humantime_serde")]
    pub testing_interval: Duration,

    /// Specifies delay between attempting to run the measurement again if the previous run failed
    /// due to being unable to get the list of nodes.
    #[serde(with = "humantime_serde")]
    pub retry_timeout: Duration,
}

impl VerlocDebug {
    const DEFAULT_PACKETS_PER_NODE: usize = 100;
    const DEFAULT_CONNECTION_TIMEOUT: Duration = Duration::from_millis(5000);
    const DEFAULT_PACKET_TIMEOUT: Duration = Duration::from_millis(1500);
    const DEFAULT_DELAY_BETWEEN_PACKETS: Duration = Duration::from_millis(50);
    const DEFAULT_BATCH_SIZE: usize = 50;
    const DEFAULT_TESTING_INTERVAL: Duration = Duration::from_secs(60 * 60 * 12);
    const DEFAULT_RETRY_TIMEOUT: Duration = Duration::from_secs(60 * 30);
}

impl Default for VerlocDebug {
    fn default() -> Self {
        VerlocDebug {
            packets_per_node: Self::DEFAULT_PACKETS_PER_NODE,
            connection_timeout: Self::DEFAULT_CONNECTION_TIMEOUT,
            packet_timeout: Self::DEFAULT_PACKET_TIMEOUT,
            delay_between_packets: Self::DEFAULT_DELAY_BETWEEN_PACKETS,
            tested_nodes_batch_size: Self::DEFAULT_BATCH_SIZE,
            testing_interval: Self::DEFAULT_TESTING_INTERVAL,
            retry_timeout: Self::DEFAULT_RETRY_TIMEOUT,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Wireguard {
    /// Specifies whether the wireguard service is enabled on this node.
    pub enabled: bool,

    /// Socket address this node will use for binding its wireguard interface.
    /// default: `[::]:51822`
    pub bind_address: SocketAddr,

    /// Private IPv4 address of the wireguard gateway.
    /// default: `10.1.0.1`
    pub private_ipv4: Ipv4Addr,

    /// Private IPv6 address of the wireguard gateway.
    /// default: `fc01::1`
    pub private_ipv6: Ipv6Addr,

    /// Port announced to external clients wishing to connect to the wireguard interface.
    /// Useful in the instances where the node is behind a proxy.
    pub announced_port: u16,

    /// The prefix denoting the maximum number of the clients that can be connected via Wireguard using IPv4.
    /// The maximum value for IPv4 is 32
    pub private_network_prefix_v4: u8,

    /// The prefix denoting the maximum number of the clients that can be connected via Wireguard using IPv6.
    /// The maximum value for IPv6 is 128
    pub private_network_prefix_v6: u8,

    /// Paths for wireguard keys, client registries, etc.
    pub storage_paths: persistence::WireguardPaths,
}

impl Wireguard {
    pub fn new_default<P: AsRef<Path>>(data_dir: P) -> Self {
        Wireguard {
            enabled: false,
            bind_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), WG_PORT),
            private_ipv4: WG_TUN_DEVICE_IP_ADDRESS_V4,
            private_ipv6: WG_TUN_DEVICE_IP_ADDRESS_V6,
            announced_port: WG_PORT,
            private_network_prefix_v4: WG_TUN_DEVICE_NETMASK_V4,
            private_network_prefix_v6: WG_TUN_DEVICE_NETMASK_V6,
            storage_paths: persistence::WireguardPaths::new(data_dir),
        }
    }
}

impl From<Wireguard> for nym_wireguard_types::Config {
    fn from(value: Wireguard) -> Self {
        nym_wireguard_types::Config {
            bind_address: value.bind_address,
            private_ipv4: value.private_ipv4,
            private_ipv6: value.private_ipv6,
            announced_port: value.announced_port,
            private_network_prefix_v4: value.private_network_prefix_v4,
            private_network_prefix_v6: value.private_network_prefix_v6,
        }
    }
}

impl From<Wireguard> for nym_authenticator::config::Authenticator {
    fn from(value: Wireguard) -> Self {
        nym_authenticator::config::Authenticator {
            bind_address: value.bind_address,
            private_ipv4: value.private_ipv4,
            private_ipv6: value.private_ipv6,
            announced_port: value.announced_port,
            private_network_prefix_v4: value.private_network_prefix_v4,
            private_network_prefix_v6: value.private_network_prefix_v6,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LocalWireguardOpts {
    #[allow(dead_code)]
    pub config: Wireguard,

    #[allow(dead_code)]
    pub custom_mixnet_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Debug {
    /// Specifies the time to live of the internal topology provider cache.
    #[serde(with = "humantime_serde")]
    pub topology_cache_ttl: Duration,

    /// Specifies the time between attempting to resolve any pending unknown nodes in the routing filter
    #[serde(with = "humantime_serde")]
    pub routing_nodes_check_interval: Duration,

    /// Specifies whether this node runs in testnet mode thus allowing it to route packets on local interfaces
    pub testnet: bool,
}

impl Debug {
    pub const DEFAULT_TOPOLOGY_CACHE_TTL: Duration = Duration::from_secs(10 * 60);
    pub const DEFAULT_ROUTING_NODES_CHECK_INTERVAL: Duration = Duration::from_secs(5 * 60);
}

impl Default for Debug {
    fn default() -> Self {
        Debug {
            topology_cache_ttl: Self::DEFAULT_TOPOLOGY_CACHE_TTL,
            routing_nodes_check_interval: Self::DEFAULT_ROUTING_NODES_CHECK_INTERVAL,
            testnet: false,
        }
    }
}

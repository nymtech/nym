// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::config::persistence::{
    EcashSignerPaths, NetworkMonitorPaths, NodeStatusAPIPaths, NymApiPaths,
};
use crate::support::config::r#override::OverrideConfig;
use crate::support::config::template::CONFIG_TEMPLATE;
use anyhow::bail;
use nym_compact_ecash::constants;
use nym_config::defaults::mainnet::read_parsed_var_if_not_default;
use nym_config::defaults::var_names::{CONFIGURED, NYXD};
use nym_config::defaults::MINIMUM_TICKETBOOK_DATA_REQUEST_SIZE;
use nym_config::serde_helpers::de_maybe_stringified;
use nym_config::{
    must_get_home, read_config_from_toml_file, save_formatted_config_to_file, NymConfigTemplate,
    DEFAULT_CONFIG_DIR, DEFAULT_CONFIG_FILENAME, DEFAULT_DATA_DIR, DEFAULT_NYM_APIS_DIR, NYM_DIR,
};
use serde::{Deserialize, Serialize};
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{debug, warn};
use url::Url;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub(crate) mod helpers;

mod r#override;
mod persistence;
mod template;
mod upgrade_helpers;

pub const DEFAULT_LOCAL_VALIDATOR: &str = "http://localhost:26657";

const DEFAULT_GATEWAY_SENDING_RATE: usize = 200;
const DEFAULT_MAX_CONCURRENT_GATEWAY_CLIENTS: usize = 50;
const DEFAULT_PACKET_DELIVERY_TIMEOUT: Duration = Duration::from_secs(20);
const DEFAULT_MONITOR_RUN_INTERVAL: Duration = Duration::from_secs(15 * 60);
// Set this to a high value for now, so that we don't risk sporadic timeouts that might cause
// bought bandwidth tokens to not have time to be spent; Once we remove the gateway from the
// bandwidth bridging protocol, we can come back to a smaller timeout value
const DEFAULT_GATEWAY_RESPONSE_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const DEFAULT_GATEWAY_CONNECTION_TIMEOUT: Duration = Duration::from_secs(15);
const DEFAULT_GATEWAY_BANDWIDTH_CLAIM_TIMEOUT: Duration = Duration::from_secs(2 * 60);

const DEFAULT_TEST_ROUTES: usize = 3;
const DEFAULT_MINIMUM_TEST_ROUTES: usize = 1;
const DEFAULT_ROUTE_TEST_PACKETS: usize = 1000;
const DEFAULT_PER_NODE_TEST_PACKETS: usize = 3;

const DEFAULT_MIXNET_CACHE_REFRESH_INTERVAL: Duration = Duration::from_secs(150);
const DEFAULT_NODE_FAMILIES_CACHE_REFRESH_INTERVAL: Duration = Duration::from_secs(600);

/// Maximum number of `block_timestamp` lookups in flight in parallel during a
/// single refresh tick.
const DEFAULT_NODE_FAMILIES_BLOCK_TIMESTAMP_FETCH_CONCURRENCY: usize = 8;

/// Number of blocks to look back when bootstrapping an average block time for
/// estimating timestamps of pruned (no longer servable) heights.
const DEFAULT_NODE_FAMILIES_BLOCK_TIME_ESTIMATION_LOOKBACK: u32 = 100;
const DEFAULT_PERFORMANCE_CONTRACT_POLLING_INTERVAL: Duration = Duration::from_secs(150);
const DEFAULT_PERFORMANCE_CONTRACT_FALLBACK_EPOCHS: u32 = 12;
const DEFAULT_PERFORMANCE_CONTRACT_RETAINED_EPOCHS: usize = 25;

pub(crate) const DEFAULT_ADDRESS_CACHE_TTL: Duration = Duration::from_secs(60 * 15);
pub(crate) const DEFAULT_ADDRESS_CACHE_CAPACITY: u64 = 1000;

pub(crate) const DEFAULT_NODE_DESCRIBE_CACHE_INTERVAL: Duration = Duration::from_secs(4500);
pub(crate) const DEFAULT_NODE_DESCRIBE_BATCH_SIZE: usize = 50;

// TODO: make it configurable
pub(crate) const DEFAULT_CHAIN_STATUS_CACHE_TTL: Duration = Duration::from_secs(30);
pub(crate) const CHAIN_STALL_THRESHOLD: Duration = Duration::from_secs(5 * 60);

// contract info is changed very infrequently (essentially once per release cycle)
// so this default is more than enough
pub(crate) const DEFAULT_CONTRACT_DETAILS_CACHE_TTL: Duration = Duration::from_secs(60 * 60);
pub(crate) const DEFAULT_NETWORK_MONITORS_CACHE_TTL: Duration = Duration::from_secs(30 * 60);

pub(crate) const DEFAULT_NODE_SIGNERS_CACHE_REFRESH_INTERVAL: Duration = Duration::from_secs(600);
pub(crate) const DEFAULT_NODE_SIGNERS_CACHE_REFRESHER_START_DELAY: Duration =
    Duration::from_secs(30);

const DEFAULT_MONITOR_THRESHOLD: u8 = 60;
const DEFAULT_MIN_MIXNODE_RELIABILITY: u8 = 50;
const DEFAULT_MIN_GATEWAY_RELIABILITY: u8 = 20;
const DEFAULT_MIN_STRESS_TESTED_NODES: f32 = 0.5;

// for now, try to use last 24h of data
const DEFAULT_MIN_STRESS_TESTING_DATA_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

// What production actually runs alongside legacy v1 routing at 0.7. Parked here rather than
// applied, since `stress_testing` ships disabled: enabling it means flipping that flag AND
// restating the routing share, because the enabled weights must sum to 1.0.
const DEFAULT_STRESS_TESTING_SCORE_WEIGHT: f64 = 0.3;

// 1.0 because legacy v1 routing is the only property enabled by DEFAULT, and the enabled weights
// must sum to one. A deployment that switches another property on restates this to make room.
const DEFAULT_LEGACY_V1_ROUTING_SCORE_WEIGHT: f64 = 1.0;

const DEFAULT_CHAIN_INTERACTIONS_PENALTY: f64 = 0.2;

// matches the stress window for now: liveness probes are low-volume, so a day of samples is what
// makes a single lossy run distinguishable from a persistently broken node
const DEFAULT_LIVENESS_DATA_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

const DEFAULT_MIN_LIVENESS_TESTED_NODES: f32 = 0.5;

// Matches the stress weight. Liveness is kept INERT by `use_liveness_data` defaulting to false,
// NOT by this being zero: a zero weight was a second gate on the same thing, and its only visible
// effect was an "enabled but does nothing" state that read as a broken feature. So this carries
// the weight liveness should have WHEN switched on, making that a single flip.
//
// Which means the flip is immediate and wants the divergence surface consulted BEFORE it, not
// after. Two populations score zero on liveness for reasons unrelated to their forwarding - nodes
// that have not ingested their agents' on-chain authorisations, and gateways not yet carrying the
// monitor-session behaviour - and enabling while either is still large penalises them for a
// rollout in progress.
const DEFAULT_LIVENESS_SCORE_WEIGHT: f64 = 0.2;

/// Derive default path to nym-api's config directory.
/// It should get resolved to `$HOME/.nym/nym-api/<id>/config`
pub fn default_config_directory<P: AsRef<Path>>(id: P) -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_NYM_APIS_DIR)
        .join(id)
        .join(DEFAULT_CONFIG_DIR)
}

/// Derive default path to nym-api's config file.
/// It should get resolved to `$HOME/.nym/nym-api/<id>/config/config.toml`
pub fn default_config_filepath<P: AsRef<Path>>(id: P) -> PathBuf {
    default_config_directory(id).join(DEFAULT_CONFIG_FILENAME)
}

/// Derive default path to nym-api's data directory where files, such as keys, are stored.
/// It should get resolved to `$HOME/.nym/nym-api/<id>/data`
pub fn default_data_directory<P: AsRef<Path>>(id: P) -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_NYM_APIS_DIR)
        .join(id)
        .join(DEFAULT_DATA_DIR)
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Config {
    // additional metadata holding on-disk location of this config file
    #[serde(skip)]
    pub(crate) save_path: Option<PathBuf>,

    pub base: Base,

    #[serde(default)]
    pub performance_provider: PerformanceProvider,

    // TODO: perhaps introduce separate 'path finder' field for all the paths and directories like we have with other configs
    pub network_monitor: NetworkMonitor,

    #[serde(default)]
    pub mixnet_contract_cache: MixnetContractCache,

    #[serde(default)]
    pub node_families_cache: NodeFamiliesCache,

    pub node_status_api: NodeStatusAPI,

    #[serde(alias = "topology_cacher")]
    #[serde(default)]
    pub describe_cache: DescribeCache,

    #[serde(default)]
    pub contracts_info_cache: ContractsInfoCache,

    #[serde(default)]
    pub network_monitors_cache: NetworkMonitorsCache,

    pub rewarding: Rewarding,

    #[serde(default)]
    pub signers_cache: SignersCache,

    #[serde(alias = "coconut_signer")]
    pub ecash_signer: EcashSigner,

    #[serde(default)]
    pub directory: DirectoryConfig,

    #[serde(skip)]
    pub address_cache: AddressCacheConfig,
}

impl NymConfigTemplate for Config {
    fn template(&self) -> &'static str {
        CONFIG_TEMPLATE
    }
}

impl Config {
    pub fn new<S: AsRef<str>>(id: S) -> Self {
        Config {
            save_path: None,
            base: Base::new_default(id.as_ref()),
            performance_provider: Default::default(),
            network_monitor: NetworkMonitor::new_default(id.as_ref()),
            mixnet_contract_cache: Default::default(),
            node_families_cache: Default::default(),
            node_status_api: NodeStatusAPI::new_default(id.as_ref()),
            describe_cache: Default::default(),
            contracts_info_cache: Default::default(),
            network_monitors_cache: Default::default(),
            rewarding: Default::default(),
            signers_cache: Default::default(),
            ecash_signer: EcashSigner::new_default(id.as_ref()),
            directory: Default::default(),
            address_cache: Default::default(),
        }
    }

    pub fn validate_and_fixup(&mut self) -> anyhow::Result<()> {
        let can_sign = self.base.mnemonic.is_some();

        if !can_sign && self.rewarding.enabled {
            bail!("can't enable rewarding without providing a mnemonic")
        }

        if !can_sign && self.ecash_signer.enabled {
            bail!("can't enable coconut signer without providing a mnemonic")
        }

        if self.base.storage_paths.persistent_cache_directory == PathBuf::default() {
            warn!("[base.storage_paths].persistent_cache_directory has not been set correctly - using default value instead");
            self.base.storage_paths.persistent_cache_directory =
                NymApiPaths::new_default(&self.base.id).persistent_cache_directory;
        }

        // FIXUPS first, so that validation below judges what the config actually means rather
        // than a half-migrated form of it
        self.performance_provider.apply_deprecated_fields();

        self.ecash_signer.validate()?;
        self.performance_provider.validate()?;
        self.directory.validate()?;

        Ok(())
    }

    pub fn override_with_args<O: Into<OverrideConfig>>(mut self, args: O) -> Self {
        let args = args.into();

        if let Some(enabled_monitor) = args.enable_monitor {
            self.network_monitor.enabled = enabled_monitor;
        }
        if let Some(enable_rewarding) = args.enable_rewarding {
            self.rewarding.enabled = enable_rewarding;
        }
        if let Some(nyxd_upstream) = args.nyxd_validator {
            self.base.local_validator = nyxd_upstream;
        }
        if let Some(bearer) = args.utility_routes_bearer {
            self.base.utility_routes_bearer = Some(bearer)
        }
        if let Some(mnemonic) = args.mnemonic {
            self.base.mnemonic = Some(mnemonic)
        }
        if let Some(enable_zk_nym) = args.enable_zk_nym {
            self.ecash_signer.enabled = enable_zk_nym
        }
        if let Some(announce_address) = args.announce_address {
            self.ecash_signer.announce_address = Some(announce_address)
        }
        if let Some(monitor_credentials_mode) = args.monitor_credentials_mode {
            self.network_monitor.debug.disabled_credentials_mode = !monitor_credentials_mode
        }
        if let Some(http_bind_address) = args.bind_address {
            self.base.bind_address = http_bind_address
        }
        if args.allow_illegal_ips {
            self.describe_cache.debug.allow_illegal_ips = true
        }
        if let Some(address_cache_ttl) = args.address_cache_ttl {
            self.address_cache.time_to_live = address_cache_ttl;
        }
        if let Some(address_cache_capacity) = args.address_cache_capacity {
            self.address_cache.capacity = address_cache_capacity;
        }

        self
    }

    pub fn override_with_env(mut self) -> Self {
        if std::env::var(CONFIGURED).is_ok() {
            // currently the only value that can be overridden is 'nyxd'
            if let Some(Ok(custom_nyxd)) = read_parsed_var_if_not_default(NYXD) {
                self.base.local_validator = custom_nyxd
            }
        }
        self
    }

    // simple wrapper that reads config file and assigns path location
    fn read_from_path<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let path = path.as_ref();
        let mut loaded: Config = read_config_from_toml_file(path)?;
        loaded.save_path = Some(path.to_path_buf());
        debug!("loaded config file from {}", path.display());
        Ok(loaded)
    }

    #[allow(dead_code)]
    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        Self::read_from_path(path)
    }

    pub fn read_from_default_path<P: AsRef<Path>>(id: P) -> io::Result<Self> {
        Self::read_from_path(default_config_filepath(id))
    }

    pub fn default_location(&self) -> PathBuf {
        default_config_filepath(&self.base.id)
    }

    pub fn save_to_default_location(&self) -> io::Result<()> {
        let config_save_location: PathBuf = self.default_location();
        save_formatted_config_to_file(self, config_save_location)
    }

    pub fn try_save(&self) -> io::Result<()> {
        if let Some(save_location) = &self.save_path {
            save_formatted_config_to_file(self, save_location)
        } else {
            debug!("config file save location is unknown. falling back to the default");
            self.save_to_default_location()
        }
    }

    pub fn get_nyxd_url(&self) -> Url {
        self.base.local_validator.clone()
    }

    pub fn get_mnemonic(&self) -> Option<&bip39::Mnemonic> {
        self.base.mnemonic.as_ref()
    }
}

fn default_http_socket_addr() -> SocketAddr {
    cfg_if::cfg_if! {
        if #[cfg(debug_assertions)] {
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8000)
        } else {
            SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8000)
        }
    }
}

// we only really care about the mnemonic being zeroized
#[derive(Debug, Deserialize, PartialEq, Eq, Serialize, Zeroize, ZeroizeOnDrop)]
pub struct Base {
    /// ID specifies the human readable ID of this particular nym-api.
    pub id: String,

    #[zeroize(skip)]
    pub local_validator: Url,

    /// Socket address Axum will use for binding its HTTP API.
    #[zeroize(skip)]
    #[serde(default = "default_http_socket_addr")]
    pub bind_address: SocketAddr,

    /// Bearer token for exposing and accessing additional utility routes
    #[serde(default)]
    #[serde(deserialize_with = "de_maybe_stringified")]
    pub utility_routes_bearer: Option<String>,

    /// Mnemonic used for rewarding and/or multisig operations
    // TODO: similarly to the note in gateway, this should get moved to a separate file
    #[serde(deserialize_with = "de_maybe_stringified")]
    mnemonic: Option<bip39::Mnemonic>,

    /// Storage paths to the common nym-api files
    #[zeroize(skip)]
    pub storage_paths: NymApiPaths,
}

impl Base {
    pub fn new_default<S: Into<String>>(id: S) -> Self {
        // SAFETY: the provided hardcoded value is well-formed
        #[allow(clippy::expect_used)]
        let default_validator: Url = DEFAULT_LOCAL_VALIDATOR
            .parse()
            .expect("default local validator is malformed!");

        let id = id.into();

        Base {
            storage_paths: NymApiPaths::new_default(&id),
            id,
            local_validator: default_validator,
            bind_address: default_http_socket_addr(),
            utility_routes_bearer: None,
            mnemonic: None,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ContractsInfoCache {
    pub time_to_live: Duration,
}

impl Default for ContractsInfoCache {
    fn default() -> Self {
        ContractsInfoCache {
            time_to_live: DEFAULT_CONTRACT_DETAILS_CACHE_TTL,
        }
    }
}

/// Configuration for the in-memory cache of authorised network-monitor orchestrators.
///
/// Controls how often nym-api re-queries the network-monitors contract for the authorised set;
/// a new orchestrator registering on-chain will not be recognised for submissions until the next
/// refresh triggered by this TTL.
#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct NetworkMonitorsCache {
    pub time_to_live: Duration,
}

impl Default for NetworkMonitorsCache {
    fn default() -> Self {
        NetworkMonitorsCache {
            time_to_live: DEFAULT_NETWORK_MONITORS_CACHE_TTL,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct MixnetContractCache {
    #[serde(default)]
    pub debug: MixnetContractCacheDebug,
}

#[allow(clippy::derivable_impls)]
impl Default for MixnetContractCache {
    fn default() -> Self {
        MixnetContractCache {
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct MixnetContractCacheDebug {
    #[serde(with = "humantime_serde")]
    pub caching_interval: Duration,
}

impl Default for MixnetContractCacheDebug {
    fn default() -> Self {
        MixnetContractCacheDebug {
            caching_interval: DEFAULT_MIXNET_CACHE_REFRESH_INTERVAL,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct NodeFamiliesCache {
    #[serde(default)]
    pub debug: NodeFamiliesCacheDebug,
}

#[allow(clippy::derivable_impls)]
impl Default for NodeFamiliesCache {
    fn default() -> Self {
        NodeFamiliesCache {
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct NodeFamiliesCacheDebug {
    #[serde(with = "humantime_serde")]
    pub caching_interval: Duration,

    pub node_families_block_timestamp_fetch_concurrency: usize,

    /// Blocks to look back when bootstrapping an average block time for
    /// estimating timestamps of pruned heights.
    pub node_families_block_time_estimation_lookback: u32,
}

impl Default for NodeFamiliesCacheDebug {
    fn default() -> Self {
        NodeFamiliesCacheDebug {
            caching_interval: DEFAULT_NODE_FAMILIES_CACHE_REFRESH_INTERVAL,
            node_families_block_timestamp_fetch_concurrency:
                DEFAULT_NODE_FAMILIES_BLOCK_TIMESTAMP_FETCH_CONCURRENCY,
            node_families_block_time_estimation_lookback:
                DEFAULT_NODE_FAMILIES_BLOCK_TIME_ESTIMATION_LOOKBACK,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct PerformanceProvider {
    /// Specifies whether this nym-api should attempt to retrieve node performance
    /// information from the performance contract.
    pub use_performance_contract_data: bool,

    /// Which properties contribute to a node's score, and in what proportion.
    #[serde(default)]
    pub scoring: PerformanceProviderScoring,

    pub debug: PerformanceProviderDebug,
}

impl PerformanceProvider {
    /// Carries the pre-move `[performance_provider.debug]` stress fields onto the scoring section,
    /// warning for each one found, and clears them.
    ///
    /// A FIXUP, not a validation: it resolves what the config means before anything checks whether
    /// that meaning is legal. Must therefore run BEFORE [`Self::validate`], which is pure.
    ///
    /// Without it the move would be SILENT, because serde ignores unknown fields: an operator who
    /// had deliberately tuned `stress_testing_score_weight` would just get the default back with
    /// no indication. A legacy key that is present WINS over the new section, so an unmigrated
    /// config keeps behaving as it did; the warnings are what tell an operator to delete it.
    #[allow(deprecated)] // the one legitimate read of these; see their deprecation notes
    pub fn apply_deprecated_fields(&mut self) {
        let legacy_enabled = self.debug.use_stress_testing_data.take();
        let legacy_weight = self.debug.stress_testing_score_weight.take();

        if let Some(enabled) = legacy_enabled {
            warn!(
                "[performance_provider.debug].use_stress_testing_data is deprecated and has moved \
                 to [performance_provider.scoring.stress_testing].enabled - applying the old \
                 value ({enabled}) for now, please migrate your config"
            );
            self.scoring.stress_testing.enabled = enabled;
        }

        if let Some(weight) = legacy_weight {
            warn!(
                "[performance_provider.debug].stress_testing_score_weight is deprecated and has \
                 moved to [performance_provider.scoring.stress_testing].weight - applying the \
                 old value ({weight}) for now, please migrate your config"
            );
            self.scoring.stress_testing.weight = weight;
        }

        // A config predating the scoring section declares no routing weight, so it sits at the
        // default 1.0. Taking a legacy `use_stress_testing_data = true` on top of that would make
        // the enabled weights sum above one and FAIL validation - an upgrade that refuses to boot
        // on a config that was previously fine. Derive routing's share instead, which reproduces
        // the split the old two-field form implied.
        if (legacy_enabled.is_some() || legacy_weight.is_some())
            && self.scoring.stress_testing.enabled
        {
            let derived = 1.0 - self.scoring.stress_testing.weight;
            warn!(
                "deriving [performance_provider.scoring.legacy_v1_routing].weight as {derived} to \
                 make room for the stress testing share taken from the deprecated fields - set \
                 both weights explicitly to silence this"
            );
            self.scoring.legacy_v1_routing.weight = derived;
        }
    }

    fn validate(&self) -> anyhow::Result<()> {
        self.scoring.validate(self.use_performance_contract_data)?;
        self.debug.validate()
    }
}

#[allow(clippy::derivable_impls)]
impl Default for PerformanceProvider {
    fn default() -> Self {
        PerformanceProvider {
            // to be changed later
            use_performance_contract_data: false,
            scoring: Default::default(),
            debug: Default::default(),
        }
    }
}

/// One property contributing to a node's score: whether it applies, and its share.
///
/// The delivery properties measure the SAME thing - whether a node carries traffic - from
/// different sources, so their weights are proportions of one figure rather than independent axes.
/// That figure is then multiplied by the node's config score, which therefore gates every property
/// equally instead of only the legacy one.
#[derive(Debug, Copy, Clone, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct ScoreProperty {
    pub enabled: bool,

    /// Share of the score. The ENABLED properties' weights must sum to 1.0. A property that does
    /// not APPLY to a given node, or that is dropped at runtime by its availability threshold, is
    /// renormalised away rather than deflating that node's score - which is how a gateway, never
    /// stress-tested, is scored on what it does have rather than penalised for what it cannot.
    pub weight: f64,
}

impl ScoreProperty {
    fn new(enabled: bool, weight: f64) -> Self {
        ScoreProperty { enabled, weight }
    }
}

impl Default for ScoreProperty {
    fn default() -> Self {
        ScoreProperty::new(false, 0.0)
    }
}

/// The delivery properties, pooled so that the sum-to-one rule ranges over a named set rather than
/// over fields scattered through the debug section.
///
/// Only `enabled` and `weight` live here. The data windows and availability thresholds stay where
/// they are: they are per-source data-quality knobs of differing shapes, and `legacy_v1_routing`
/// has neither today, so folding them in would give one property fields it ignores.
#[derive(Debug, Copy, Clone, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct PerformanceProviderScoring {
    /// The network monitor v1 routing score. Enabled by default, being the historical basis of the
    /// performance score, and disableable so that v1 can eventually be RETIRED - at which point
    /// liveness must be carrying the measurement in its place, which the validation enforces.
    pub legacy_v1_routing: ScoreProperty,

    pub stress_testing: ScoreProperty,

    pub liveness: ScoreProperty,
}

impl Default for PerformanceProviderScoring {
    fn default() -> Self {
        PerformanceProviderScoring {
            legacy_v1_routing: ScoreProperty::new(true, DEFAULT_LEGACY_V1_ROUTING_SCORE_WEIGHT),
            // NOTE: left DISABLED to preserve the shipped default, which has always been
            // `use_stress_testing_data: false`. Its weight matches what production actually runs,
            // so enabling it is one flag flip plus restating the routing share to 0.7.
            stress_testing: ScoreProperty::new(false, DEFAULT_STRESS_TESTING_SCORE_WEIGHT),
            liveness: ScoreProperty::new(false, DEFAULT_LIVENESS_SCORE_WEIGHT),
        }
    }
}

impl PerformanceProviderScoring {
    /// Every property, paired with the name to use when complaining about it.
    fn named(&self) -> [(&'static str, ScoreProperty); 3] {
        [
            ("legacy_v1_routing", self.legacy_v1_routing),
            ("stress_testing", self.stress_testing),
            ("liveness", self.liveness),
        ]
    }

    fn enabled(&self) -> impl Iterator<Item = (&'static str, ScoreProperty)> {
        self.named().into_iter().filter(|(_, p)| p.enabled)
    }

    fn validate(&self, use_performance_contract_data: bool) -> anyhow::Result<()> {
        for (name, property) in self.named() {
            if property.weight < 0.0 || property.weight > 1.0 || !property.weight.is_finite() {
                bail!(
                    "[performance_provider.scoring.{name}].weight is set to a value outside of \
                     the range [0.0, 1.0]"
                );
            }
        }

        // the contract provider serves none of these, so enabling one alongside it would ask for
        // a measurement that provider structurally cannot supply
        if use_performance_contract_data {
            if let Some((name, _)) = self.enabled().next() {
                bail!(
                    "[performance_provider.scoring.{name}] cannot be enabled while \
                     [performance_provider].use_performance_contract_data is also enabled"
                );
            }
            return Ok(());
        }

        // a node's delivery score is the weighted mean of whatever applies to it, so with nothing
        // enabled there is no measurement to score at all and nym-api would publish no performance
        let enabled: Vec<_> = self.enabled().filter(|(_, p)| p.weight > 0.0).collect();
        if enabled.is_empty() {
            bail!(
                "at least one [performance_provider.scoring.*] property must be enabled with a \
                 non-zero weight, otherwise no node can be assigned a performance score"
            );
        }

        // stress testing covers mixnodes ONLY, so it cannot be the sole property: every gateway
        // would then have an empty applied set and no definable score. Routing covers every node
        // and liveness covers gateways too, so requiring one of them keeps the score total.
        if !self.legacy_v1_routing.enabled && !self.liveness.enabled {
            bail!(
                "either [performance_provider.scoring.legacy_v1_routing] or \
                 [performance_provider.scoring.liveness] must be enabled: stress testing applies \
                 to mixnodes alone, so on its own it leaves gateways unscoreable"
            );
        }

        // The weights are proportions of ONE measurement, so the enabled ones must account for all
        // of it. A property that does not apply to a given node, or that its availability
        // threshold drops at runtime, is renormalised away instead - which is why this checks the
        // enabled set rather than the applied one.
        //
        // Compared with a tolerance rather than `!= 1.0`, because binary floats make the exact
        // test reject clean configurations: 0.7 + 0.2 + 0.1 is 0.9999999999999999, so a 70/20/10
        // split would fail at startup while 0.34/0.33/0.33 sums to exactly 1.0 and passes. It is
        // also order-dependent, which would make validity hinge on the order these are folded in.
        // The tolerance is tight enough to still reject operator error: 0.33 three times is 0.99.
        let total: f64 = enabled.iter().map(|(_, p)| p.weight).sum();
        if (total - 1.0).abs() > 1e-9 {
            let listed = enabled
                .iter()
                .map(|(name, p)| format!("{name}={}", p.weight))
                .collect::<Vec<_>>()
                .join(", ");
            bail!(
                "the enabled [performance_provider.scoring.*] weights must sum to 1.0, got \
                 {total} ({listed})"
            );
        }

        Ok(())
    }
}

// the deprecated fields below are still (de)serialised, and the derives read them, so the
// expansion has to be exempt. Placed on the struct rather than on the fields so that reads from
// ANYWHERE ELSE still warn - which is the point of deprecating them.
#[allow(deprecated)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct PerformanceProviderDebug {
    /// Specifies interval of polling the performance contract. Note it is only applicable
    /// if the contract data is being used.
    /// Further note that if there have been no updates to the cache, the performance overhead is negligible
    /// (i.e. there will be only a single query performed to check if anything has changed)
    #[serde(with = "humantime_serde")]
    pub contract_polling_interval: Duration,

    /// Specify the maximum number of epochs we can fallback to if given epoch's performance data
    /// is not available in the contract
    pub max_performance_fallback_epochs: u32,

    /// Specify the maximum number of epoch entries to be kept in the cache in case we needed non-current data
    // (currently we need an equivalent of full day worth of data for legacy endpoints)
    pub max_epoch_entries_to_retain: usize,

    /// If `stress_testing` is enabled, this specifies the minimum % of nodes,
    /// that must have their stress data available in the `stress_testing_data_period`,
    /// in order to include that metric in performance calculation.
    /// This is done to protect against Network Monitor failures and not receiving any data.
    pub minimum_available_stress_testing_results: f32,

    /// Moved to `[performance_provider.scoring.stress_testing].enabled`. Still read so that a
    /// config predating the move is not silently ignored: if present it is applied over the new
    /// value with a warning. Remove once operators have migrated.
    #[serde(default)]
    #[deprecated(
        since = "1.1.88",
        note = "moved to [performance_provider.scoring.stress_testing].enabled; read only by \
                PerformanceProvider::apply_deprecated_fields"
    )]
    pub use_stress_testing_data: Option<bool>,

    /// Moved to `[performance_provider.scoring.stress_testing].weight`. Read on the same terms as
    /// `use_stress_testing_data` above.
    #[serde(default)]
    #[deprecated(
        since = "1.1.88",
        note = "moved to [performance_provider.scoring.stress_testing].weight; read only by \
                PerformanceProvider::apply_deprecated_fields"
    )]
    pub stress_testing_score_weight: Option<f64>,

    /// Config score penalty for nodes that do not have a cosmos account capable of interacting with the nyx chain
    pub chain_interactions_penalty: f64,

    /// Specifies the duration of the rolling average used for stress testing score.
    #[serde(with = "humantime_serde")]
    pub stress_testing_data_period: Duration,

    /// Specifies the duration of the rolling average used for the liveness score.
    /// Kept separate from `stress_testing_data_period` because the two kinds are probed on their
    /// own cadences, so one window length need not suit both.
    #[serde(with = "humantime_serde")]
    pub liveness_data_period: Duration,

    /// If `liveness` is enabled, this specifies the minimum % of liveness-eligible
    /// nodes that must have their liveness data available in the `liveness_data_period`,
    /// in order to include that metric in performance calculation.
    /// This is done to protect against Network Monitor failures and not receiving any data.
    pub minimum_available_liveness_results: f32,
}

impl PerformanceProviderDebug {
    pub fn validate(&self) -> anyhow::Result<()> {
        // the score weights moved to [performance_provider.scoring] and are range-checked there
        if self.chain_interactions_penalty < 0.0
            || self.chain_interactions_penalty > 1.0
            || !self.chain_interactions_penalty.is_finite()
        {
            bail!("the .chain_interactions_penalty field is set to a value outside of the range [0.0, 1.0]");
        }
        Ok(())
    }
}

#[allow(clippy::derivable_impls)]
// initialising the deprecated fields to `None` is the whole point of them defaulting to absent
#[allow(deprecated)]
impl Default for PerformanceProviderDebug {
    fn default() -> Self {
        PerformanceProviderDebug {
            contract_polling_interval: DEFAULT_PERFORMANCE_CONTRACT_POLLING_INTERVAL,
            max_performance_fallback_epochs: DEFAULT_PERFORMANCE_CONTRACT_FALLBACK_EPOCHS,
            max_epoch_entries_to_retain: DEFAULT_PERFORMANCE_CONTRACT_RETAINED_EPOCHS,

            minimum_available_stress_testing_results: DEFAULT_MIN_STRESS_TESTED_NODES,
            chain_interactions_penalty: DEFAULT_CHAIN_INTERACTIONS_PENALTY,
            stress_testing_data_period: DEFAULT_MIN_STRESS_TESTING_DATA_INTERVAL,
            liveness_data_period: DEFAULT_LIVENESS_DATA_INTERVAL,
            minimum_available_liveness_results: DEFAULT_MIN_LIVENESS_TESTED_NODES,

            // deprecated, resolved onto [performance_provider.scoring] by
            // `PerformanceProvider::apply_deprecated_fields` before validation
            use_stress_testing_data: None,
            stress_testing_score_weight: None,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct SignersCache {
    pub enabled: bool,

    pub debug: SignersCacheDebug,
}

impl Default for SignersCache {
    fn default() -> Self {
        SignersCache {
            enabled: true,
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct SignersCacheDebug {
    // TODO: make it into a decaying function so that if multiple signers are down,
    // the refresh interval would decrease
    #[serde(with = "humantime_serde")]
    pub refresh_interval: Duration,

    // give it some time so that the actual api on THIS singer could start
    // and it wouldn't self-report itself as being down
    #[serde(with = "humantime_serde")]
    pub refresher_start_delay: Duration,
}

impl Default for SignersCacheDebug {
    fn default() -> Self {
        SignersCacheDebug {
            refresh_interval: DEFAULT_NODE_SIGNERS_CACHE_REFRESH_INTERVAL,
            refresher_start_delay: DEFAULT_NODE_SIGNERS_CACHE_REFRESHER_START_DELAY,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct AddressCacheConfig {
    pub time_to_live: Duration,
    pub capacity: u64,
}

impl Default for AddressCacheConfig {
    fn default() -> Self {
        Self {
            time_to_live: DEFAULT_ADDRESS_CACHE_TTL,
            capacity: DEFAULT_ADDRESS_CACHE_CAPACITY,
        }
    }
}

// this got separated into 2 structs so that we could have a sane `default` implementation for the latter
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct NetworkMonitor {
    /// Specifies whether network monitoring service is enabled in this process.
    pub enabled: bool,

    pub storage_paths: NetworkMonitorPaths,

    #[serde(default)]
    pub debug: NetworkMonitorDebug,
}

impl NetworkMonitor {
    pub fn new_default<P: AsRef<Path>>(id: P) -> Self {
        NetworkMonitor {
            enabled: false,
            storage_paths: NetworkMonitorPaths::new_default(id),
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct NetworkMonitorDebug {
    //  Mixnodes and gateways with reliability lower the this get blacklisted by network monitor, get no traffic and cannot be selected into a rewarded set.
    pub min_mixnode_reliability: u8, // defaults to 50
    pub min_gateway_reliability: u8, // defaults to 20

    /// Indicates whether this validator api is running in a disabled credentials mode, thus attempting
    /// to claim bandwidth without presenting bandwidth credentials.
    pub disabled_credentials_mode: bool,

    /// Specifies the interval at which the network monitor sends the test packets.
    #[serde(with = "humantime_serde")]
    pub run_interval: Duration,

    /// Specifies maximum rate (in packets per second) of test packets being sent to gateway
    pub gateway_sending_rate: usize,

    /// Maximum number of gateway clients the network monitor will try to talk to concurrently.
    /// 0 = no limit
    pub max_concurrent_gateway_clients: usize,

    /// Maximum allowed time for receiving gateway response.
    #[serde(with = "humantime_serde")]
    pub gateway_response_timeout: Duration,

    /// Maximum allowed time for the gateway connection to get established.
    #[serde(with = "humantime_serde")]
    pub gateway_connection_timeout: Duration,

    /// Maximum allowed time for the gateway bandwidth claim to get resolved
    #[serde(with = "humantime_serde")]
    pub gateway_bandwidth_claim_timeout: Duration,

    /// Specifies the duration the monitor is going to wait after sending all measurement
    /// packets before declaring nodes unreachable.
    #[serde(with = "humantime_serde")]
    pub packet_delivery_timeout: Duration,

    /// Desired number of test routes to be constructed (and working) during a monitor test run.
    pub test_routes: usize,

    /// The minimum number of test routes that need to be constructed (and working) in order for
    /// a monitor test run to be valid.
    pub minimum_test_routes: usize,

    /// Number of test packets sent via each pseudorandom route to verify whether they work correctly,
    /// before using them for testing the rest of the network.
    pub route_test_packets: usize,

    /// Number of test packets sent to each node during regular monitor test run.
    pub per_node_test_packets: usize,
}

impl Default for NetworkMonitorDebug {
    fn default() -> Self {
        NetworkMonitorDebug {
            min_mixnode_reliability: DEFAULT_MIN_MIXNODE_RELIABILITY,
            min_gateway_reliability: DEFAULT_MIN_GATEWAY_RELIABILITY,
            disabled_credentials_mode: true,
            run_interval: DEFAULT_MONITOR_RUN_INTERVAL,
            gateway_sending_rate: DEFAULT_GATEWAY_SENDING_RATE,
            max_concurrent_gateway_clients: DEFAULT_MAX_CONCURRENT_GATEWAY_CLIENTS,
            gateway_response_timeout: DEFAULT_GATEWAY_RESPONSE_TIMEOUT,
            gateway_connection_timeout: DEFAULT_GATEWAY_CONNECTION_TIMEOUT,
            gateway_bandwidth_claim_timeout: DEFAULT_GATEWAY_BANDWIDTH_CLAIM_TIMEOUT,
            packet_delivery_timeout: DEFAULT_PACKET_DELIVERY_TIMEOUT,
            test_routes: DEFAULT_TEST_ROUTES,
            minimum_test_routes: DEFAULT_MINIMUM_TEST_ROUTES,
            route_test_packets: DEFAULT_ROUTE_TEST_PACKETS,
            per_node_test_packets: DEFAULT_PER_NODE_TEST_PACKETS,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct NodeStatusAPI {
    // pub enabled: bool,
    pub storage_paths: NodeStatusAPIPaths,

    #[serde(default)]
    pub debug: NodeStatusAPIDebug,
}

impl NodeStatusAPI {
    pub fn new_default<P: AsRef<Path>>(id: P) -> Self {
        NodeStatusAPI {
            storage_paths: NodeStatusAPIPaths::new_default(id),
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct NodeStatusAPIDebug {
    // TODO: allow for this...
    // port: u16,
    #[serde(with = "humantime_serde")]
    pub caching_interval: Duration,

    // base amount (in unym)
    pub minimum_on_chain_balance_amount: u128,

    pub chain_capabilities_retrieval_concurrency: usize,

    #[serde(with = "humantime_serde")]
    pub chain_capabilities_refresh_interval: Duration,
}

impl NodeStatusAPIDebug {
    const DEFAULT_NODE_STATUS_CACHE_REFRESH_INTERVAL: Duration = Duration::from_secs(305);
    const DEFAULT_CHAIN_CAPABILITIES_RETRIEVAL_CONCURRENCY: usize = 8;
    const DEFAULT_CHAIN_CAPABILITIES_REFRESH_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60); // once a day is more than enough
    const DEFAULT_MINIMUM_ON_CHAIN_BALANCE: u128 = 1_000000; // 1 nym is enough for all tx fees for quite some time
}

impl Default for NodeStatusAPIDebug {
    fn default() -> Self {
        NodeStatusAPIDebug {
            caching_interval: Self::DEFAULT_NODE_STATUS_CACHE_REFRESH_INTERVAL,
            minimum_on_chain_balance_amount: Self::DEFAULT_MINIMUM_ON_CHAIN_BALANCE,
            chain_capabilities_retrieval_concurrency:
                Self::DEFAULT_CHAIN_CAPABILITIES_RETRIEVAL_CONCURRENCY,
            chain_capabilities_refresh_interval: Self::DEFAULT_CHAIN_CAPABILITIES_REFRESH_INTERVAL,
        }
    }
}

#[derive(Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct DescribeCache {
    // pub enabled: bool,

    // pub paths: TopologyCacherPathfinder,
    #[serde(default)]
    pub debug: DescribeCacheDebug,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct DescribeCacheDebug {
    #[serde(with = "humantime_serde")]
    #[serde(alias = "node_describe_caching_interval")]
    pub caching_interval: Duration,

    #[serde(alias = "node_describe_batch_size")]
    pub batch_size: usize,

    #[serde(alias = "node_describe_allow_illegal_ips")]
    pub allow_illegal_ips: bool,
}

impl Default for DescribeCacheDebug {
    fn default() -> Self {
        DescribeCacheDebug {
            caching_interval: DEFAULT_NODE_DESCRIBE_CACHE_INTERVAL,
            batch_size: DEFAULT_NODE_DESCRIBE_BATCH_SIZE,
            allow_illegal_ips: false,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Rewarding {
    /// Specifies whether rewarding service is enabled in this process.
    pub enabled: bool,

    // this should really be a thing too...
    // pub paths: RewardingPathfinder,
    #[serde(default)]
    pub debug: RewardingDebug,
}

#[allow(clippy::derivable_impls)]
impl Default for Rewarding {
    fn default() -> Self {
        Rewarding {
            enabled: false,
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct RewardingDebug {
    /// Specifies the minimum percentage of monitor test run data present in order to
    /// distribute rewards for given interval.
    /// Note, only values in range 0-100 are valid
    pub minimum_interval_monitor_threshold: u8,
}

impl Default for RewardingDebug {
    fn default() -> Self {
        RewardingDebug {
            minimum_interval_monitor_threshold: DEFAULT_MONITOR_THRESHOLD,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct EcashSigner {
    /// Specifies whether rewarding service is enabled in this process.
    pub enabled: bool,

    #[serde(deserialize_with = "de_maybe_stringified")]
    pub announce_address: Option<Url>,

    pub storage_paths: EcashSignerPaths,

    #[serde(default)]
    pub debug: EcashSignerDebug,
}

impl EcashSigner {
    pub fn new_default<P: AsRef<Path>>(id: P) -> Self {
        EcashSigner {
            enabled: false,
            announce_address: None,
            storage_paths: EcashSignerPaths::new_default(id),
            debug: Default::default(),
        }
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.debug.maximum_size_of_data_request < MINIMUM_TICKETBOOK_DATA_REQUEST_SIZE {
            bail!("the .maximum_size_of_data_request field is set to a lower value than the minimum value in the system ({MINIMUM_TICKETBOOK_DATA_REQUEST_SIZE})");
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct EcashSignerDebug {
    /// Duration of the interval for polling the dkg contract.
    #[serde(with = "humantime_serde")]
    pub dkg_contract_polling_rate: Duration,

    /// Specifies interval at which the stale ecash data is removed from the storage.
    #[serde(with = "humantime_serde")]
    pub stale_data_cleaner_interval: Duration,

    /// How long a cached view of the DKG epoch may be served before it is refreshed, and hence
    /// the window over which signers can disagree about whether a ceremony has concluded.
    #[serde(with = "humantime_serde")]
    pub epoch_cache_staleness: Duration,

    /// How long an epoch that has just stopped being issuable is still accepted on issuance
    /// requests that explicitly ask for it.
    ///
    /// Signers learn of a ceremony concluding from a cache, so for a short while they disagree
    /// about which epoch is issuable. This window covers that disagreement, and MUST therefore
    /// stay above `epoch_cache_staleness` - otherwise a client collecting across the changeover
    /// can have some signers sign for it and the rest refuse for good.
    #[serde(with = "humantime_serde")]
    pub issuance_grace_period: Duration,

    /// Specifies how long should the issued ticketbooks be kept (beyond the specified expiration date)
    pub issued_ticketbooks_retention_period_days: u32,

    /// Specifies how long should the full ticket data of verified gateway tickets be kept (beyond the spending date)
    pub verified_tickets_retention_period_days: u32,

    /// Specifies how many partial ticketbooks the api is willing to return in a single request.
    pub maximum_size_of_data_request: usize,
}

impl EcashSignerDebug {
    pub const DEFAULT_DKG_CONTRACT_POLLING_RATE: Duration = Duration::from_secs(30);

    // it still operates at "day" cutoffs
    pub const DEFAULT_STALE_DATA_CLEANER_INTERVAL: Duration = Duration::from_secs(2 * 60 * 60);

    pub const DEFAULT_EPOCH_CACHE_STALENESS: Duration = Duration::from_secs(5 * 60);

    // twice the window over which signers can disagree, so that every one of them has seen a
    // concluded ceremony before any stops honouring the epoch it replaced. Derived from the
    // staleness itself, since that is what the window is compensating for.
    pub const DEFAULT_ISSUANCE_GRACE_PERIOD: Duration =
        Duration::from_secs(2 * Self::DEFAULT_EPOCH_CACHE_STALENESS.as_secs());

    // keep them for 2 extra days beyond the specified expiration date
    pub(crate) const DEFAULT_MAX_ISSUED_TICKETBOOKS_RETENTION_DAYS: u32 = 2;

    // keep the tickets for maximum theoretical validity (+1 day)
    pub(crate) const DEFAULT_VERIFIED_TICKETS_RETENTION_PERIOD_DAYS: u32 =
        constants::CRED_VALIDITY_PERIOD_DAYS + 1;

    pub const MAXIMUM_SIZE_OF_DATA_REQUEST: usize = 100;
}

impl Default for EcashSignerDebug {
    fn default() -> Self {
        EcashSignerDebug {
            dkg_contract_polling_rate: Self::DEFAULT_DKG_CONTRACT_POLLING_RATE,
            stale_data_cleaner_interval: Self::DEFAULT_STALE_DATA_CLEANER_INTERVAL,
            epoch_cache_staleness: Self::DEFAULT_EPOCH_CACHE_STALENESS,
            issuance_grace_period: Self::DEFAULT_ISSUANCE_GRACE_PERIOD,
            issued_ticketbooks_retention_period_days:
                Self::DEFAULT_MAX_ISSUED_TICKETBOOKS_RETENTION_DAYS,
            verified_tickets_retention_period_days:
                Self::DEFAULT_VERIFIED_TICKETS_RETENTION_PERIOD_DAYS,
            maximum_size_of_data_request: Self::MAXIMUM_SIZE_OF_DATA_REQUEST,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize, Default)]
#[serde(default)]
pub struct DirectoryConfig {
    pub debug: DirectoryConfigDebug,
}

impl DirectoryConfig {
    fn validate(&self) -> anyhow::Result<()> {
        self.debug.validate()
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct DirectoryConfigDebug {
    /// Number of snapshots to keep
    pub retention_count: usize,

    /// Number of blocks to wait before promoting the most recently pulled snapshot as latest.
    pub settle_lag: usize,

    /// Specifies whether the RPC node this api is connected to is trusted.
    /// It controls method of anchoring directory trust.
    pub trusted_rpc_node: bool,

    /// How often the chain should be polled for the current height
    /// and consequently for whether new snapshot should be taken
    #[serde(with = "humantime_serde")]
    pub polling_interval: Duration,
}

impl DirectoryConfigDebug {
    pub const DEFAULT_RETENTION_COUNT: usize = 3;
    pub const DEFAULT_SETTLE_LAG: usize = 10;
    pub const DEFAULT_POLLING_INTERVAL: Duration = Duration::from_secs(30);

    fn validate(&self) -> anyhow::Result<()> {
        if !self.trusted_rpc_node {
            bail!("untrusted local rpc node is currently not fully supported")
        }
        Ok(())
    }
}

impl Default for DirectoryConfigDebug {
    fn default() -> Self {
        DirectoryConfigDebug {
            retention_count: Self::DEFAULT_RETENTION_COUNT,
            settle_lag: Self::DEFAULT_SETTLE_LAG,
            trusted_rpc_node: true,
            polling_interval: Self::DEFAULT_POLLING_INTERVAL,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The upgrade path that matters: a config written before `[performance_provider.scoring]`
    /// existed, with stress testing switched on the old way, must still boot. Routing's share is
    /// derived so the enabled weights sum to one instead of failing validation.
    #[test]
    #[allow(deprecated)] // constructing the legacy form is the point
    fn an_unmigrated_config_with_stress_enabled_still_validates() {
        let mut provider = PerformanceProvider::default();
        provider.debug.use_stress_testing_data = Some(true);
        provider.debug.stress_testing_score_weight = Some(0.3);

        provider.apply_deprecated_fields();

        assert!(provider.scoring.stress_testing.enabled);
        assert_eq!(provider.scoring.stress_testing.weight, 0.3);
        assert_eq!(provider.scoring.legacy_v1_routing.weight, 0.7);
        assert!(provider.validate().is_ok());

        // and the legacy fields are consumed, so a second pass is a no-op
        assert!(provider.debug.use_stress_testing_data.is_none());
        assert!(provider.debug.stress_testing_score_weight.is_none());
    }

    /// The old flag absent, or present-but-false, must leave routing alone at its full share.
    #[test]
    #[allow(deprecated)]
    fn an_unmigrated_config_without_stress_keeps_routing_whole() {
        let mut provider = PerformanceProvider::default();
        provider.debug.use_stress_testing_data = Some(false);
        provider.debug.stress_testing_score_weight = Some(0.3);

        provider.apply_deprecated_fields();

        assert!(!provider.scoring.stress_testing.enabled);
        assert_eq!(provider.scoring.legacy_v1_routing.weight, 1.0);
        assert!(provider.validate().is_ok());
    }

    #[test]
    fn a_default_config_validates() {
        assert!(PerformanceProvider::default().validate().is_ok());
    }

    /// Stress alone leaves every gateway unscoreable, which is why one of the two node-wide
    /// properties must always be on.
    #[test]
    fn stress_testing_cannot_be_the_only_enabled_property() {
        let mut provider = PerformanceProvider::default();
        provider.scoring.legacy_v1_routing.enabled = false;
        provider.scoring.stress_testing = ScoreProperty::new(true, 1.0);

        let err = provider.validate().unwrap_err().to_string();
        assert!(err.contains("gateways unscoreable"), "got: {err}");
    }

    #[test]
    fn enabled_weights_must_sum_to_one() {
        let mut provider = PerformanceProvider::default();
        provider.scoring.stress_testing = ScoreProperty::new(true, 0.3);
        // routing still at 1.0, so the enabled weights sum to 1.3
        let err = provider.validate().unwrap_err().to_string();
        assert!(err.contains("must sum to 1.0"), "got: {err}");
    }
}

#[cfg(test)]
mod template_tests {
    use super::*;
    use nym_config::NymConfigTemplate;

    /// The template renders at `init` time rather than compile time, so a mistyped variable would
    /// otherwise first surface as a panic in front of an operator creating a config.
    #[test]
    fn the_config_template_renders_and_carries_the_scoring_section() {
        let rendered = Config::new("template-test").format_to_string();

        for expected in [
            "[performance_provider.scoring.legacy_v1_routing]",
            "[performance_provider.scoring.stress_testing]",
            "[performance_provider.scoring.liveness]",
        ] {
            assert!(rendered.contains(expected), "missing {expected}");
        }

        // the moved keys must not be advertised any more, or a fresh config would be born
        // carrying deprecated fields and warning on every start
        for gone in ["use_stress_testing_data", "use_liveness_data"] {
            assert!(!rendered.contains(gone), "template still emits {gone}");
        }

        // and what it renders must parse back into an equivalent config
        let parsed: Config = toml::from_str(&rendered).expect("rendered template must parse");
        assert_eq!(
            parsed.performance_provider.scoring,
            Config::new("template-test").performance_provider.scoring
        );
    }
}

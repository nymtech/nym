// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::config::persistence::{EcashSignerPaths, NodeStatusAPIPaths, NymApiPaths};
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

const DEFAULT_MIXNET_CACHE_REFRESH_INTERVAL: Duration = Duration::from_secs(150);
const DEFAULT_NODE_FAMILIES_CACHE_REFRESH_INTERVAL: Duration = Duration::from_secs(600);

/// Maximum number of `block_timestamp` lookups in flight in parallel during a
/// single refresh tick.
const DEFAULT_NODE_FAMILIES_BLOCK_TIMESTAMP_FETCH_CONCURRENCY: usize = 8;

/// Number of blocks to look back when bootstrapping an average block time for
/// estimating timestamps of pruned (no longer servable) heights.
const DEFAULT_NODE_FAMILIES_BLOCK_TIME_ESTIMATION_LOOKBACK: u32 = 100;

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

pub(crate) const DEFAULT_NODE_SIGNERS_CACHE_REFRESH_INTERVAL: Duration = Duration::from_secs(600);
pub(crate) const DEFAULT_NODE_SIGNERS_CACHE_REFRESHER_START_DELAY: Duration =
    Duration::from_secs(30);

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
            mixnet_contract_cache: Default::default(),
            node_families_cache: Default::default(),
            node_status_api: NodeStatusAPI::new_default(id.as_ref()),
            describe_cache: Default::default(),
            contracts_info_cache: Default::default(),
            signers_cache: Default::default(),
            ecash_signer: EcashSigner::new_default(id.as_ref()),
            directory: Default::default(),
            address_cache: Default::default(),
        }
    }

    pub fn validate_and_fixup(&mut self) -> anyhow::Result<()> {
        let can_sign = self.base.mnemonic.is_some();

        if !can_sign && self.ecash_signer.enabled {
            bail!("can't enable coconut signer without providing a mnemonic")
        }

        if self.base.storage_paths.persistent_cache_directory == PathBuf::default() {
            warn!("[base.storage_paths].persistent_cache_directory has not been set correctly - using default value instead");
            self.base.storage_paths.persistent_cache_directory =
                NymApiPaths::new_default(&self.base.id).persistent_cache_directory;
        }

        self.ecash_signer.validate()?;
        self.directory.validate()?;

        Ok(())
    }

    pub fn override_with_args<O: Into<OverrideConfig>>(mut self, args: O) -> Self {
        let args = args.into();

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

    /// Mnemonic used for the ecash signer's DKG and multisig operations
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

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct NodeStatusAPI {
    // pub enabled: bool,
    pub storage_paths: NodeStatusAPIPaths,
}

impl NodeStatusAPI {
    pub fn new_default<P: AsRef<Path>>(id: P) -> Self {
        NodeStatusAPI {
            storage_paths: NodeStatusAPIPaths::new_default(id),
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

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct EcashSigner {
    /// Specifies whether ecash signing protocol is enabled in this process.
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

/// Shared by every attested contract's snapshots, not only the directory's.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct DirectoryConfigDebug {
    /// Number of snapshots to keep.
    pub retention_count: usize,

    /// Number of blocks to wait before promoting the most recently pulled snapshot as latest.
    pub settle_lag: usize,

    /// Specifies whether the RPC node this api is connected to is trusted, which decides how
    /// chain trust is anchored: a proven read through it, or a light client.
    pub trusted_rpc_node: bool,

    /// How often the chain should be polled for the current height
    /// and consequently for whether new snapshot should be taken.
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
mod template_tests {
    use super::*;
    use nym_config::NymConfigTemplate;

    /// The template renders at `init` time rather than compile time, so a mistyped variable would
    /// otherwise first surface as a panic in front of an operator creating a config.
    #[test]
    fn the_config_template_renders_and_parses_back() {
        let rendered = Config::new("template-test").format_to_string();

        // a fresh config must not be born carrying a section nothing reads
        for gone in [
            "[network_monitor]",
            "[network_monitor.",
            "[performance_provider",
        ] {
            assert!(!rendered.contains(gone), "template still emits {gone}");
        }

        toml::from_str::<Config>(&rendered).expect("rendered template must parse");
    }
}

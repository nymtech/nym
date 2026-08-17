// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use clap::Parser;
use nym_bin_common::bin_info;
use nym_validator_client::nyxd::bip39;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;
use url::Url;

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

#[derive(Debug, clap::Args)]
pub(crate) struct HttpArgs {
    /// Bearer token for accessing the http endpoints.
    #[clap(
        long,
        env = "NYM_GEOLOCATOR_HTTP_AUTH_TOKEN",
        alias = "http-bearer-token"
    )]
    pub(crate) http_auth_token: String,

    #[clap(
        long,
        env = "NYM_GEOLOCATOR_HTTP_BIND_ADDRESS",
        default_value = "[::]:8080"
    )]
    pub(crate) bind_address: SocketAddr,

    /// How long a node-signed re-test request stays valid after it was signed.
    #[clap(long, env = "NYM_GEOLOCATOR_RETEST_REQUEST_VALIDITY_WINDOW", value_parser = humantime::parse_duration, default_value = "30s")]
    pub(crate) retest_request_validity_window: Duration,

    /// How many consecutive node-requested measurements may return an unchanged location before
    /// that node is put into cooldown.
    #[clap(
        long,
        env = "NYM_GEOLOCATOR_RETEST_BURST_THRESHOLD",
        default_value_t = 3
    )]
    pub(crate) retest_burst_threshold: u32,

    /// How long a node that has spent its re-test allowance must wait.
    #[clap(long, env = "NYM_GEOLOCATOR_RETEST_BURST_COOLDOWN", value_parser = humantime::parse_duration, default_value = "7days")]
    pub(crate) retest_burst_cooldown: Duration,
}

#[derive(Debug, clap::Args)]
pub(crate) struct ScraperArgs {
    #[clap(
        long,
        default_value = "2h",
        env = "NYM_GEOLOCATOR_NODE_REFRESH_INTERVAL"
    )]
    #[clap(value_parser = humantime::parse_duration)]
    pub(crate) node_refresh_interval: Duration,

    /// Timeout for querying a single node for its detailed information (ip addresses) (e.g. `10s`).
    #[clap(long, env = "NYM_GEOLOCATOR_NODE_INFO_QUERY_TIMEOUT", value_parser = humantime::parse_duration, default_value = "10s")]
    pub(crate) node_info_query_timeout: Duration,

    /// Maximum number of nodes queried concurrently during a node refresh cycle.
    #[clap(
        long,
        env = "NYM_GEOLOCATOR_CONCURRENT_NODE_QUERIES",
        default_value_t = 16
    )]
    pub(crate) number_of_concurrent_node_queries: usize,

    /// Maximum number of addresses a node may announce before its details are rejected outright.
    #[clap(
        long,
        env = "NYM_GEOLOCATOR_MAX_ADDRESSES_PER_NODE",
        default_value_t = 3
    )]
    pub(crate) max_addresses_per_node: usize,
}

#[derive(Debug, clap::Args)]
pub(crate) struct GeolocationArgs {
    #[clap(long, default_value = "30days", env = "NYM_GEOLOCATOR_GEODATA_TTL")]
    #[clap(value_parser = humantime::parse_duration)]
    pub(crate) geodata_ttl: Duration,

    #[clap(
        long,
        default_value = "1h",
        env = "NYM_GEOLOCATOR_EXPIRATION_POLLING_INTERVAL"
    )]
    #[clap(value_parser = humantime::parse_duration)]
    pub(crate) expiration_polling_interval: Duration,

    #[clap(
        long,
        default_value = "1h",
        env = "NYM_GEOLOCATOR_IPINFO_LOOKUP_CACHE_TTL"
    )]
    #[clap(value_parser = humantime::parse_duration)]
    pub(crate) ip_info_lookup_cache_ttl: Duration,

    /// Maximum number of nodes measured in a single sweep, bounding the cold-start burst.
    #[clap(
        long,
        env = "NYM_GEOLOCATOR_MAX_NODES_MEASURED_PER_SWEEP",
        default_value_t = 250
    )]
    pub(crate) max_nodes_measured_per_sweep: usize,

    /// Maximum number of addresses sent to the lookup provider in a single request.
    #[clap(
        long,
        env = "NYM_GEOLOCATOR_MAX_ADDRESSES_PER_LOOKUP",
        default_value_t = 100
    )]
    pub(crate) max_addresses_per_lookup: usize,

    /// https://github.com/ipinfo/rust
    #[clap(long, env = "NYM_GEOLOCATOR_IPINFO_API_TOKEN")]
    pub(crate) ipinfo_api_token: String,
}

#[derive(Debug, clap::Args)]
pub(crate) struct ChainArgs {
    /// Nyxd address.
    #[clap(long, env = "NYM_GEOLOCATOR_NYXD")]
    pub(crate) nyxd_addr: Url,

    /// Specifies the mnemonic authorised for making deposits for the ticketbooks
    #[clap(long, env = "NYM_GEOLOCATOR_MNEMONIC")]
    pub(crate) mnemonic: bip39::Mnemonic,

    #[clap(
        long,
        default_value = "30min",
        env = "NYM_GEOLOCATOR_BOND_REFRESH_INTERVAL"
    )]
    #[clap(value_parser = humantime::parse_duration)]
    pub(crate) bond_refresh_interval: Duration,
}

#[derive(Debug, Parser)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub(crate) struct Args {
    /// Path pointing to an env file that configures the binary.
    /// Useful in local testing setups against networks different from mainnet
    #[clap(short, long)]
    pub(crate) config_env_file: Option<PathBuf>,

    #[clap(flatten)]
    pub(crate) http: HttpArgs,

    #[clap(flatten)]
    pub(crate) scraper: ScraperArgs,

    #[clap(flatten)]
    pub(crate) geolocation: GeolocationArgs,

    #[clap(flatten)]
    pub(crate) chain: ChainArgs,
}

impl Args {
    pub(crate) fn to_config(&self) -> Config {
        Config {
            described_node_refresh_interval: self.scraper.node_refresh_interval,
            number_of_concurrent_node_queries: self.scraper.number_of_concurrent_node_queries,
            node_info_query_timeout: self.scraper.node_info_query_timeout,
            max_addresses_per_node: self.scraper.max_addresses_per_node,
            geolocation_data_ttl: self.geolocation.geodata_ttl,
            ip_info_lookup_cache_ttl: self.geolocation.ip_info_lookup_cache_ttl,
            max_addresses_per_lookup: self.geolocation.max_addresses_per_lookup,
            bonded_nodes_refresh_interval: self.chain.bond_refresh_interval,
            geolocation_expiration_polling_interval: self.geolocation.expiration_polling_interval,
            max_nodes_measured_per_sweep: self.geolocation.max_nodes_measured_per_sweep,
            retest_request_validity_window: self.http.retest_request_validity_window,
            retest_burst_threshold: self.http.retest_burst_threshold,
            retest_burst_cooldown: self.http.retest_burst_cooldown,
        }
    }
}

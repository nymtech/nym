// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use clap::Parser;
use nym_bin_common::bin_info;
use nym_validator_client::nyxd::bip39;
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
}

#[derive(Debug, clap::Args)]
pub(crate) struct GeolocationArgs {
    #[clap(long, default_value = "30days", env = "NYM_GEOLOCATOR_GEODATA_TTL")]
    #[clap(value_parser = humantime::parse_duration)]
    pub(crate) geodata_ttl: Duration,

    #[clap(
        long,
        default_value = "1h",
        env = "NYM_GEOLOCATOR_IPINFO_LOOKUP_CACHE_TTL"
    )]
    #[clap(value_parser = humantime::parse_duration)]
    pub(crate) ip_info_lookup_cache_ttl: Duration,

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
}

// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use clap::Parser;
use nym_bin_common::bin_info;
use nym_http_api_client::reqwest::Url;
use nym_validator_client::nyxd::bip39;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

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
    #[arg(value_parser = humantime::parse_duration)]
    pub(crate) node_refresh_interval: Duration,
}

#[derive(Debug, clap::Args)]
pub(crate) struct GeolocationArgs {
    #[clap(long, default_value = "30days", env = "NYM_GEOLOCATOR_GEODATA_TTL")]
    #[arg(value_parser = humantime::parse_duration)]
    pub(crate) geodata_ttl: Duration,

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
    #[arg(value_parser = humantime::parse_duration)]
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

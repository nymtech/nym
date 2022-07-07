// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::config::Config;
use clap::{Parser, Subcommand};
use network_defaults::DEFAULT_NETWORK;
use url::Url;

pub mod init;
pub(crate) mod run;
pub(crate) mod upgrade;

#[cfg(not(feature = "coconut"))]
pub(crate) const DEFAULT_ETH_ENDPOINT: &str =
    "https://rinkeby.infura.io/v3/00000000000000000000000000000000";
#[cfg(not(feature = "coconut"))]
pub(crate) const DEFAULT_ETH_PRIVATE_KEY: &str =
    "0000000000000000000000000000000000000000000000000000000000000001";

fn long_version() -> String {
    format!(
        r#"
{:<20}{}
{:<20}{}
{:<20}{}
{:<20}{}
{:<20}{}
{:<20}{}
{:<20}{}
{:<20}{}
{:<20}{}
"#,
        "Build Timestamp:",
        env!("VERGEN_BUILD_TIMESTAMP"),
        "Build Version:",
        env!("VERGEN_BUILD_SEMVER"),
        "Commit SHA:",
        env!("VERGEN_GIT_SHA"),
        "Commit Date:",
        env!("VERGEN_GIT_COMMIT_TIMESTAMP"),
        "Commit Branch:",
        env!("VERGEN_GIT_BRANCH"),
        "rustc Version:",
        env!("VERGEN_RUSTC_SEMVER"),
        "rustc Channel:",
        env!("VERGEN_RUSTC_CHANNEL"),
        "cargo Profile:",
        env!("VERGEN_CARGO_PROFILE"),
        "Network:",
        DEFAULT_NETWORK
    )
}

fn long_version_static() -> &'static str {
    Box::leak(long_version().into_boxed_str())
}

#[derive(Parser)]
#[clap(author = "Nymtech", version, long_version = long_version_static(), about)]
pub(crate) struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Initialise a Nym client. Do this first!
    Init(init::Init),
    /// Run the Nym client with provided configuration client optionally overriding set parameters
    Run(run::Run),
    /// Try to upgrade the client
    Upgrade(upgrade::Upgrade),
}

// Configuration that can be overridden.
pub(crate) struct OverrideConfig {
    validators: Option<String>,
    port: Option<u16>,
    fastmode: bool,

    #[cfg(all(feature = "eth", not(feature = "coconut")))]
    enabled_credentials_mode: bool,

    #[cfg(all(feature = "eth", not(feature = "coconut")))]
    eth_private_key: Option<String>,

    #[cfg(all(feature = "eth", not(feature = "coconut")))]
    eth_endpoint: Option<String>,
}

pub(crate) async fn execute(args: &Cli) {
    match &args.command {
        Commands::Init(m) => init::execute(m).await,
        Commands::Run(m) => run::execute(m).await,
        Commands::Upgrade(m) => upgrade::execute(m),
    }
}

fn parse_validators(raw: &str) -> Vec<Url> {
    raw.split(',')
        .map(|raw_validator| {
            raw_validator
                .trim()
                .parse()
                .expect("one of the provided validator api urls is invalid")
        })
        .collect()
}

pub(crate) fn override_config(mut config: Config, args: OverrideConfig) -> Config {
    if let Some(raw_validators) = args.validators {
        config
            .get_base_mut()
            .set_custom_validator_apis(parse_validators(&raw_validators));
    }

    if let Some(port) = args.port {
        config = config.with_port(port);
    }

    #[cfg(all(not(feature = "eth"), not(feature = "coconut")))]
    {
        config
            .get_base_mut()
            .with_eth_endpoint(DEFAULT_ETH_ENDPOINT.to_string());
        config
            .get_base_mut()
            .with_eth_private_key(DEFAULT_ETH_PRIVATE_KEY.to_string());
    }

    #[cfg(all(feature = "eth", not(feature = "coconut")))]
    {
        if args.enabled_credentials_mode {
            config.get_base_mut().with_disabled_credentials(false)
        }
        if let Some(eth_endpoint) = args.eth_endpoint {
            config.get_base_mut().with_eth_endpoint(eth_endpoint);
        }
        if let Some(eth_private_key) = args.eth_private_key {
            config.get_base_mut().with_eth_private_key(eth_private_key);
        }
    }

    if args.fastmode {
        config.get_base_mut().set_high_default_traffic_volume();
    }

    config
}

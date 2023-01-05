// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::config::{BaseConfig, Config};
use build_information::BinaryBuildInformation;
use clap::CommandFactory;
use clap::{Parser, Subcommand};
use completions::{fig_generate, ArgShell};
use config::OptionalSet;
use lazy_static::lazy_static;
use std::error::Error;

pub mod init;
pub(crate) mod run;
pub(crate) mod upgrade;

lazy_static! {
    pub static ref PRETTY_BUILD_INFORMATION: String =
        BinaryBuildInformation::new(env!("CARGO_PKG_VERSION")).pretty_print();
}

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    &PRETTY_BUILD_INFORMATION
}

#[derive(Parser)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub(crate) struct Cli {
    /// Path pointing to an env file that configures the client.
    #[clap(short, long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

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

    /// Generate shell completions
    Completions(ArgShell),

    /// Generate Fig specification
    GenerateFigSpec,
}

// Configuration that can be overridden.
pub(crate) struct OverrideConfig {
    nym_apis: Option<Vec<url::Url>>,
    port: Option<u16>,
    use_anonymous_replies: bool,
    fastmode: bool,
    no_cover: bool,

    #[cfg(feature = "coconut")]
    nyxd_urls: Option<Vec<url::Url>>,
    #[cfg(feature = "coconut")]
    enabled_credentials_mode: bool,
}

pub(crate) async fn execute(args: &Cli) -> Result<(), Box<dyn Error + Send + Sync>> {
    let bin_name = "nym-socks5-client";

    match &args.command {
        Commands::Init(m) => init::execute(m).await?,
        Commands::Run(m) => run::execute(m).await?,
        Commands::Upgrade(m) => upgrade::execute(m),
        Commands::Completions(s) => s.generate(&mut Cli::command(), bin_name),
        Commands::GenerateFigSpec => fig_generate(&mut Cli::command(), bin_name),
    }
    Ok(())
}

pub(crate) fn override_config(mut config: Config, args: OverrideConfig) -> Config {
    config = config
        .with_base(BaseConfig::with_high_default_traffic_volume, args.fastmode)
        .with_base(BaseConfig::with_disabled_cover_traffic, args.no_cover)
        .with_anonymous_replies(args.use_anonymous_replies)
        .with_optional(Config::with_port, args.port)
        .with_optional_custom_env_ext(
            BaseConfig::with_custom_nym_apis,
            args.nym_apis,
            network_defaults::var_names::NYM_API,
            config::parse_urls,
        );

    #[cfg(feature = "coconut")]
    {
        config = config
            .with_optional_custom_env_ext(
                BaseConfig::with_custom_nyxd,
                args.nymd_validators,
                network_defaults::var_names::NYXD,
                config::parse_urls,
            )
            .with_base(
                BaseConfig::with_disabled_credentials,
                !args.enabled_credentials_mode,
            );
    }

    config
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert();
    }
}

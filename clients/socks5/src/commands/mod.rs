// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::config::Config;
use build_information::BinaryBuildInformation;
use clap::CommandFactory;
use clap::{Parser, Subcommand};
use completions::{fig_generate, ArgShell};
use config::parse_urls;
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
    nymd_validators: Option<Vec<url::Url>>,
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
    if let Some(nym_apis) = args.nym_apis {
        config.get_base_mut().set_custom_nym_apis(nym_apis);
    } else if let Ok(raw_validators) = std::env::var(network_defaults::var_names::API_VALIDATOR) {
        config
            .get_base_mut()
            .set_custom_nym_apis(parse_urls(&raw_validators));
    }

    if args.use_anonymous_replies {
        config = config.with_anonymous_replies(true)
    }

    if let Some(port) = args.port {
        config = config.with_port(port);
    }

    #[cfg(feature = "coconut")]
    {
        if let Some(nymd_validators) = args.nymd_validators {
            config.get_base_mut().set_custom_validators(nymd_validators);
        } else if let Ok(raw_validators) =
            std::env::var(network_defaults::var_names::NYMD_VALIDATOR)
        {
            config
                .get_base_mut()
                .set_custom_validators(parse_urls(&raw_validators));
        }
        if args.enabled_credentials_mode {
            config.get_base_mut().with_disabled_credentials(false)
        }
    }

    if args.fastmode {
        config.get_base_mut().set_high_default_traffic_volume();
    }

    if args.no_cover {
        config.get_base_mut().set_no_cover_traffic();
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

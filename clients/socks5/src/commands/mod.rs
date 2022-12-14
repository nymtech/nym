// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::error::Error;

use crate::client::config::Config;
use clap::CommandFactory;
use clap::{Parser, Subcommand};
use completions::{fig_generate, ArgShell};
use config::parse_validators;

pub mod init;
pub(crate) mod run;
pub(crate) mod upgrade;

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
    )
}

fn long_version_static() -> &'static str {
    Box::leak(long_version().into_boxed_str())
}

#[derive(Parser)]
#[clap(author = "Nymtech", version, long_version = long_version_static(), about)]
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
    nymd_validators: Option<String>,
    api_validators: Option<String>,
    port: Option<u16>,
    use_anonymous_sender_tag: bool,
    fastmode: bool,
    no_cover: bool,

    #[cfg(feature = "coconut")]
    enabled_credentials_mode: bool,
}

pub(crate) async fn execute(args: &Cli) -> Result<(), Box<dyn Error + Send + Sync>> {
    let bin_name = "nym-socks5-client";

    match &args.command {
        Commands::Init(m) => init::execute(m).await?,
        Commands::Run(m) => run::execute(m).await?,
        Commands::Upgrade(m) => upgrade::execute(m),
        Commands::Completions(s) => s.generate(&mut Cli::into_app(), bin_name),
        Commands::GenerateFigSpec => fig_generate(&mut Cli::into_app(), bin_name),
    }
    Ok(())
}

pub(crate) fn override_config(mut config: Config, args: OverrideConfig) -> Config {
    if let Some(raw_validators) = args.nymd_validators {
        config
            .get_base_mut()
            .set_custom_validators(parse_validators(&raw_validators));
    } else if let Ok(raw_validators) = std::env::var(network_defaults::var_names::NYMD_VALIDATOR) {
        config
            .get_base_mut()
            .set_custom_validators(parse_validators(&raw_validators));
    }
    if let Some(raw_validators) = args.api_validators {
        config
            .get_base_mut()
            .set_custom_validator_apis(parse_validators(&raw_validators));
    } else if let Ok(raw_validators) = std::env::var(network_defaults::var_names::API_VALIDATOR) {
        config
            .get_base_mut()
            .set_custom_validator_apis(parse_validators(&raw_validators));
    }

    if args.use_anonymous_sender_tag {
        config = config.with_anonymous_replies(true)
    }

    if let Some(port) = args.port {
        config = config.with_port(port);
    }

    #[cfg(feature = "coconut")]
    {
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

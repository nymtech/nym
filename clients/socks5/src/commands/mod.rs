// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::config::Config;
use clap::{Parser, Subcommand};
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
    #[clap(long)]
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
}

// Configuration that can be overridden.
pub(crate) struct OverrideConfig {
    validators: Option<String>,
    port: Option<u16>,
    fastmode: bool,

    #[cfg(feature = "coconut")]
    enabled_credentials_mode: bool,
}

pub(crate) async fn execute(args: &Cli) {
    match &args.command {
        Commands::Init(m) => init::execute(m).await,
        Commands::Run(m) => run::execute(m).await,
        Commands::Upgrade(m) => upgrade::execute(m),
    }
}

pub(crate) fn override_config(mut config: Config, args: OverrideConfig) -> Config {
    if let Some(raw_validators) = args.validators {
        config
            .get_base_mut()
            .set_custom_validator_apis(parse_validators(&raw_validators));
    } else if let Ok(raw_validators) = std::env::var(network_defaults::var_names::API_VALIDATOR) {
        config
            .get_base_mut()
            .set_custom_validator_apis(parse_validators(&raw_validators));
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

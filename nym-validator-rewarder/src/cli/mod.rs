// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::error::NymRewarderError;
use clap::{Parser, Subcommand};
use nym_bin_common::bin_info;
use std::path::PathBuf;
use std::sync::OnceLock;
use tracing::{debug, error};

pub mod build_info;
pub mod init;
pub mod run;
pub mod upgrade_helpers;

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

#[derive(Parser, Debug)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub(crate) struct Cli {
    /// Path pointing to an env file that configures the validator rewarder and overrides any preconfigured values.
    #[clap(short, long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    /// Flag used for disabling the printed banner in tty.
    #[clap(long)]
    pub(crate) no_banner: bool,

    #[clap(subcommand)]
    command: Commands,
}

impl Cli {
    pub(crate) async fn execute(self) -> Result<(), NymRewarderError> {
        match self.command {
            Commands::Init(args) => init::execute(args),
            Commands::Run(args) => run::execute(args).await,
            Commands::BuildInfo(args) => build_info::execute(args),
        }
    }
}

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Initialise a validator rewarder with persistent config.toml file.
    Init(init::Args),

    /// Run the validator rewarder with the preconfigured settings.
    Run(run::Args),

    /// Show build information of this binary
    BuildInfo(build_info::Args),
}

fn try_load_current_config(custom_path: &Option<PathBuf>) -> Result<Config, NymRewarderError> {
    let config_path = custom_path.clone().unwrap_or(Config::default_location());

    debug!(
        "attempting to load configuration file from {}",
        config_path.display()
    );

    if let Ok(cfg) = Config::read_from_toml_file(&config_path) {
        return Ok(cfg);
    }

    upgrade_helpers::try_upgrade_config(&config_path)?;

    Config::read_from_toml_file(&config_path).map_err(|err| {
        error!(
            "Failed to load config. Are you sure you have run `init` before? (Error was: {err})",
        );
        err
    })
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

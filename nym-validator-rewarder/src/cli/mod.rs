// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::error::NymRewarderError;
use clap::{Parser, Subcommand};
use nym_bin_common::bin_info;
use nym_validator_client::nyxd::{AccountId, Coin};
use std::path::PathBuf;
use std::sync::OnceLock;
use tracing::{debug, error};
use url::Url;

pub mod build_info;
pub mod init;
pub mod process_block;
pub mod process_until;
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
            Commands::ProcessBlock(args) => process_block::execute(args).await,
            Commands::ProcessUntil(args) => process_until::execute(args).await,
            Commands::BuildInfo(args) => build_info::execute(args),
        }
    }
}

#[derive(Debug, clap::Args)]
pub struct ConfigOverridableArgs {
    #[clap(long, env = "NYM_VALIDATOR_REWARDER_DISABLE_BLOCK_SIGNING_REWARDING")]
    pub disable_block_signing_rewarding: bool,

    #[clap(long, env = "NYM_VALIDATOR_REWARDER_BLOCK_SIGNING_MONITORING_ONLY")]
    pub block_signing_monitoring_only: bool,

    #[clap(long, env = "NYM_VALIDATOR_TICKETBOOK_ISSUANCE_MONITORING_ONLY")]
    pub ticketbook_issuance_monitoring_only: bool,

    #[clap(
        long,
        env = "NYM_VALIDATOR_REWARDER_DISABLE_TICKETBOOK_ISSUANCE_REWARDING"
    )]
    pub disable_ticketbook_issuance_rewarding: bool,

    #[clap(long, env = "NYM_VALIDATOR_REWARDER_SCRAPER_ENDPOINT")]
    pub scraper_endpoint: Option<Url>,

    #[clap(long, env = "NYM_VALIDATOR_REWARDER_NYXD_ENDPOINT")]
    pub nyxd_endpoint: Option<Url>,

    #[clap(long, env = "NYM_VALIDATOR_REWARDER_EPOCH_BUDGET")]
    pub epoch_budget: Option<Coin>,

    #[clap(long, env = "NYM_VALIDATOR_REWARDER_EPOCH_DURATION")]
    pub epoch_duration: Option<humantime::Duration>,

    #[clap(long, env = "NYM_VALIDATOR_REWARDER_BLOCK_SIGNING_REWARD_RATIO")]
    pub block_signing_reward_ratio: Option<f64>,

    #[clap(long, env = "NYM_VALIDATOR_REWARDER_TICKETBOOK_ISSUANCE_REWARD_RATIO")]
    pub ticketbook_issuance_reward_ratio: Option<f64>,

    #[clap(
        long,
        value_delimiter = ',',
        env = "NYM_VALIDATOR_REWARDER_BLOCK_SIGNING_WHITELIST"
    )]
    pub block_signing_whitelist: Option<Vec<AccountId>>,

    #[clap(
        long,
        value_delimiter = ',',
        env = "NYM_VALIDATOR_REWARDER_ISSUANCE_MONITOR_WHITELIST"
    )]
    pub issuance_monitor_whitelist: Option<Vec<AccountId>>,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Initialise a validator rewarder with persistent config.toml file.
    Init(init::Args),

    /// Run the validator rewarder with the preconfigured settings.
    Run(run::Args),

    /// Attempt to process a single block.
    ProcessBlock(process_block::Args),

    /// Attempt to process multiple blocks until the provided height.
    ProcessUntil(process_until::Args),

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
        cfg.validate()?;
        return Ok(cfg);
    }

    upgrade_helpers::try_upgrade_config(&config_path)?;

    let config = Config::read_from_toml_file(&config_path).map_err(|err| {
        error!(
            "Failed to load config. Are you sure you have run `init` before? (Error was: {err})",
        );
        err
    })?;
    config.validate()?;
    Ok(config)
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

// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::{default_config_filepath, Config};
use crate::env::{setup_env, Env};
use crate::error::NymvisorError;
use clap::{Parser, Subcommand};
use lazy_static::lazy_static;
use nym_bin_common::bin_info;
use std::path::Path;
use tracing::error;

mod add_upgrade;
mod build_info;
mod config;
mod daemon_build_info;
pub(crate) mod helpers;
mod init;
mod run;

lazy_static! {
    pub static ref PRETTY_BUILD_INFORMATION: String = bin_info!().pretty_print();
}

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    &PRETTY_BUILD_INFORMATION
}

#[derive(Parser, Debug)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub(crate) struct Cli {
    /// Path pointing to an env file that configures the nymvisor and overrides any preconfigured values.
    #[clap(short, long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    #[clap(subcommand)]
    command: Commands,
}

impl Cli {
    pub(crate) fn execute(self) -> Result<(), NymvisorError> {
        setup_env(&self.config_env_file)?;

        match self.command {
            Commands::Init(args) => init::execute(*args),
            Commands::Run(args) => run::execute(args),
            Commands::BuildInfo(args) => build_info::execute(args),
            Commands::DaemonBuildInfo(args) => daemon_build_info::execute(args),
            Commands::AddUpgrade(args) => add_upgrade::execute(args),
            Commands::Config(args) => config::execute(args),
        }
    }
}

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Initialise a nymvisor instance with persistent Config.toml file.
    Init(Box<init::Args>),

    /// Run the associated daemon with the preconfigured settings.
    Run(run::Args),

    /// Show build information of this binary
    BuildInfo(build_info::Args),

    /// Show build information of the associated daemon
    DaemonBuildInfo(daemon_build_info::Args),

    /// Queues up another upgrade for the associated daemon
    AddUpgrade(add_upgrade::Args),

    /// Show configuration options being used by this instance of nymvisor
    Config(config::Args),
}

fn open_config_file(env: &Env) -> Result<Config, NymvisorError> {
    let config_load_location = if let Some(config_path) = &env.nymvisor_config_path {
        config_path.clone()
    } else {
        // if no explicit path was provided in the environment, try to infer it with other vars
        let id = env.try_nymvisor_id()?;
        default_config_filepath(id)
    };

    if let Ok(cfg) = Config::read_from_toml_file(&config_load_location) {
        return Ok(cfg);
    }

    // we couldn't load it - try upgrading it from older revisions
    try_upgrade_config(&config_load_location)?;

    match Config::read_from_toml_file(&config_load_location) {
        Ok(cfg) => Ok(cfg),
        Err(source) => {
            error!("Failed to load config from {}. Are you sure you have run `init` before? (Error was: {source})", config_load_location.display());
            Err(NymvisorError::ConfigLoadFailure {
                id: env.try_nymvisor_id().unwrap_or_default(),
                path: config_load_location,
                source,
            })
        }
    }
}

pub(crate) fn try_load_current_config(env: &Env) -> Result<Config, NymvisorError> {
    let mut config = open_config_file(env)?;
    env.override_config(&mut config);
    Ok(config)
}

// no upgrades for now
fn try_upgrade_config<P: AsRef<Path>>(_config_location: P) -> Result<(), NymvisorError> {
    Ok(())
}

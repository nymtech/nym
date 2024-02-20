// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::{default_config_filepath, default_instances_directory, Config};
use crate::env::{setup_env, Env};
use crate::error::NymvisorError;
use clap::{Parser, Subcommand};
use nym_bin_common::bin_info;
use nym_config::{DEFAULT_CONFIG_DIR, DEFAULT_CONFIG_FILENAME};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tracing::{debug, error};

mod add_upgrade;
mod build_info;
mod config;
mod daemon_build_info;
pub(crate) mod helpers;
mod init;
mod run;

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
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
    } else if let Some(nymvisor_id) = &env.nymvisor_id {
        // if no explicit path was provided in the environment, try to use the default one based on the nymvisor id
        default_config_filepath(nymvisor_id)
    } else {
        // finally, if all else fails, see if this is a singleton -> if so try to load the only instance
        try_get_singleton_nymvisor_config_path()?
    };

    debug!(
        "attempting to load configuration file from {}",
        config_load_location.display()
    );

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
                id: env.nymvisor_id.clone().unwrap_or("UNKNOWN".to_string()),
                path: config_load_location,
                source,
            })
        }
    }
}

// attempt to get a path to nymvisor's config path if there is only a single instance
pub(crate) fn try_get_singleton_nymvisor_config_path() -> Result<PathBuf, NymvisorError> {
    let instances_dir = default_instances_directory();
    let mut instances = instances_dir
        .read_dir()
        .map_err(|source| NymvisorError::InstancesReadFailure {
            source,
            path: instances_dir.clone(),
        })?
        .collect::<Vec<_>>();

    if instances.len() != 1 {
        return Err(NymvisorError::NotSingleton {
            instances: instances.len(),
        });
    }

    // safety: that unwrap is fine as we've just checked we have 1 entry in the vector
    #[allow(clippy::unwrap_used)]
    let instance_dir = instances
        .pop()
        .unwrap()
        .map_err(|source| NymvisorError::InstancesReadFailure {
            source,
            path: instances_dir,
        })?
        .path();

    // join the instance directory with `/config/config.toml`
    Ok(instance_dir
        .join(DEFAULT_CONFIG_DIR)
        .join(DEFAULT_CONFIG_FILENAME))
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

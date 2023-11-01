// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

mod add_upgrade;
mod build_info;
mod config;
mod init;
mod run;

use crate::env::setup_env;
use crate::error::NymvisorError;
use clap::{Parser, Subcommand};
use lazy_static::lazy_static;
use nym_bin_common::bin_info;

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
            Commands::Init(args) => init::execute(args),
            Commands::Run(args) => run::execute(args),
            Commands::BuildInfo(args) => build_info::execute(args),
            Commands::AddUpgrade(args) => add_upgrade::execute(args),
            Commands::Config(args) => config::execute(args),
        }
    }
}

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// TODO: document the command
    Init(init::Args),

    /// TODO: document the command
    Run(run::Args),

    /// TODO: document the command
    BuildInfo(build_info::Args),

    /// TODO: document the command
    AddUpgrade(add_upgrade::Args),

    /// TODO: document the command
    Config(config::Args),
}

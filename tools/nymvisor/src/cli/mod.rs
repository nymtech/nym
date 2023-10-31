// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

mod add_upgrade;
mod build_info;
mod config;
mod init;
mod run;

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
    // I doubt we're gonna need any global flags here, but I'm going to leave the the option open
    // so that'd be easier to add them later if needed
    #[clap(subcommand)]
    command: Commands,
}

impl Cli {
    pub(crate) fn execute(self) -> anyhow::Result<()> {
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

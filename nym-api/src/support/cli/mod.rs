// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::Result;
use clap::{Parser, Subcommand};
use nym_bin_common::bin_info;
use std::sync::OnceLock;

pub(crate) mod build_info;
pub(crate) mod init;
pub(crate) mod run;

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

#[derive(Parser, Debug)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub(crate) struct Cli {
    /// Path pointing to an env file that configures the Nym API.
    #[clap(short, long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    /// A no-op flag included for consistency with other binaries (and compatibility with nymvisor, oops)
    #[clap(long)]
    pub(crate) no_banner: bool,

    #[clap(subcommand)]
    pub(crate) command: Commands,
}

impl Cli {
    pub(crate) async fn execute(self) -> Result<(), anyhow::Error> {
        match self.command {
            Commands::Init(args) => init::execute(args).await,
            Commands::Run(args) => run::execute(args).await,
            Commands::BuildInfo(args) => build_info::execute(args),
        }
    }
}

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Initialise a Nym Api instance with persistent config.toml file.
    Init(init::Args),

    /// Run the Nym Api with provided configuration optionally overriding set parameters
    Run(run::Args),

    /// Show build information of this binary
    BuildInfo(build_info::Args),
}

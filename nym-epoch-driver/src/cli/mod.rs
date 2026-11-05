// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use clap::{Parser, Subcommand};
use nym_bin_common::bin_info;
use std::path::PathBuf;
use std::sync::OnceLock;

mod build_info;
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
    /// Path pointing to an env file that configures the network the driver runs against.
    #[clap(short, long)]
    pub(crate) config_env_file: Option<PathBuf>,

    /// Flag used for disabling the printed banner in tty.
    #[clap(long)]
    pub(crate) no_banner: bool,

    #[clap(subcommand)]
    command: Commands,
}

impl Cli {
    pub(crate) async fn execute(self) -> anyhow::Result<()> {
        match self.command {
            Commands::Init(args) => init::execute(args),
            Commands::Run(args) => run::execute(args).await,
            Commands::BuildInfo(args) => build_info::execute(args),
        }
    }
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialise the epoch driver with a persistent config.toml file.
    Init(init::Args),

    /// Run the epoch driver with its persistent configuration.
    Run(run::Args),

    /// Show build information of this binary
    BuildInfo(build_info::Args),
}

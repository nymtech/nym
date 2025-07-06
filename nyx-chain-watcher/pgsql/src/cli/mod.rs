// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::commands::{build_info, init, run};
use crate::env::vars::*;
use crate::error::NyxChainWatcherError;
use clap::{Parser, Subcommand};
use nym_bin_common::bin_info;
use std::sync::OnceLock;

mod commands;

pub const DEFAULT_NYX_CHAIN_WATCHER_ID: &str = "default-nyx-chain-watcher";

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

#[derive(Parser, Debug)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub(crate) struct Cli {
    /// Path pointing to an env file that configures the nym-chain-watcher and overrides any preconfigured values.
    #[clap(
        short,
        long,
        env = NYX_CHAIN_WATCHER_CONFIG_ENV_FILE_ARG
    )]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    /// Flag used for disabling the printed banner in tty.
    #[clap(
        long,
        env = NYX_CHAIN_WATCHER_NO_BANNER_ARG
    )]
    pub(crate) no_banner: bool,

    /// Port to listen on
    #[arg(long, default_value_t = 8000, env = "NYX_CHAIN_WATCHER_HTTP_PORT")]
    pub(crate) http_port: u16,

    #[clap(subcommand)]
    command: Commands,
}

impl Cli {
    pub(crate) async fn execute(self) -> Result<(), NyxChainWatcherError> {
        match self.command {
            Commands::BuildInfo(args) => build_info::execute(args),
            Commands::Run(args) => run::execute(*args, self.http_port).await,
            Commands::Init(args) => init::execute(args).await,
        }
    }
}

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Show build information of this binary
    BuildInfo(build_info::Args),

    /// Start this nym-chain-watcher
    Run(Box<run::Args>),

    /// Initialise config
    Init(init::Args),
}

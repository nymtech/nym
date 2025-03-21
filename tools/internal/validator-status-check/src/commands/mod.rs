// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use clap::{Parser, Subcommand};
use nym_bin_common::bin_info;
use std::path::PathBuf;
use std::sync::OnceLock;

mod build_info;
mod check_network;
mod check_signer;

fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

#[derive(Parser, Debug)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub struct Cli {
    /// Path pointing to an env file that configures the CLI.
    #[clap(short, long)]
    pub(crate) config_env_file: Option<PathBuf>,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Check status of an individual signer
    CheckSigner(check_signer::Args),

    /// Check status of all signers
    CheckNetwork(check_network::Args),

    /// Show build information of this binary
    BuildInfo(build_info::Args),
}

impl Cli {
    pub async fn execute(self) -> anyhow::Result<()> {
        match self.command {
            Commands::CheckSigner(args) => check_signer::execute(args).await?,
            Commands::CheckNetwork(args) => check_network::execute(args).await?,
            Commands::BuildInfo(args) => build_info::execute(args),
        }

        Ok(())
    }
}

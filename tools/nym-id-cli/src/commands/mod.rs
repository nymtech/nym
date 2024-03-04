// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

mod build_info;
mod import_credential;
mod setup;

use clap::{Parser, Subcommand};
use nym_bin_common::bin_info;
use std::sync::OnceLock;

fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

#[derive(Parser)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub(crate) struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

impl Cli {
    pub async fn execute(self) -> anyhow::Result<()> {
        match self.command {
            Commands::ImportCredential(args) => import_credential::execute(args).await?,
            Commands::BuildInfo(args) => build_info::execute(args),
        }

        Ok(())
    }
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    // TODO: to be determined how it's going to work in nymvpn et al.
    // ///
    // Setup,
    /// Attempt to import a bandwidth credential into the provided storage.
    ImportCredential(import_credential::Args),

    /// Show build information of this binary
    BuildInfo(build_info::Args),
}

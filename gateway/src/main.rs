// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]

use clap::{crate_name, crate_version, Parser};
use colored::Colorize;
use nym_bin_common::bin_info;
use nym_bin_common::logging::{maybe_print_banner, setup_logging};
use nym_network_defaults::setup_env;
use std::io::IsTerminal;
use std::sync::OnceLock;
use tracing::error;

mod commands;

fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

#[derive(Parser)]
#[clap(author = "Nymtech", version, about, long_version = pretty_build_info_static())]
struct Cli {
    /// Path pointing to an env file that configures the gateway.
    #[clap(short, long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    /// Force run the binary bypassing the deprecation in favour of nym-node
    #[clap(long, hide = true)]
    pub(crate) force_run: bool,

    /// Flag used for disabling the printed banner in tty.
    #[clap(long)]
    pub(crate) no_banner: bool,

    #[clap(subcommand)]
    command: commands::Commands,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_logging();

    let args = Cli::parse();
    setup_env(args.config_env_file.as_ref());

    if !args.no_banner {
        maybe_print_banner(crate_name!(), crate_version!());
    }

    commands::execute(args).await.map_err(|err| {
        if std::io::stdout().is_terminal() {
            let error_message = format!("{err}").red();
            error!("{error_message}");
            error!("Exiting...");
        }

        err
    })
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

// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use ::nym_config::defaults::setup_env;
use clap::{crate_name, crate_version, Parser};
use log::info;
use nym_bin_common::bin_info;
use nym_metrics::MetricsController;
use std::sync::OnceLock;

#[allow(unused_imports)]
use nym_bin_common::logging::{maybe_print_banner, setup_logging};
#[cfg(feature = "cpucycles")]
use nym_bin_common::setup_tracing;
#[cfg(feature = "cpucycles")]
use nym_mixnode_common::measure;
#[cfg(feature = "cpucycles")]
use tracing::instrument;

lazy_static::lazy_static! {
    pub static ref REGISTRY: MetricsController = MetricsController::default();
}

mod commands;
mod config;
pub(crate) mod error;
mod node;

fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

#[derive(Parser)]
#[clap(author = "Nymtech", version, about, long_version = pretty_build_info_static())]
struct Cli {
    /// Path pointing to an env file that configures the mixnode.
    #[clap(short, long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    /// Flag used for disabling the printed banner in tty.
    #[clap(long)]
    pub(crate) no_banner: bool,

    #[clap(subcommand)]
    command: commands::Commands,
}

#[cfg(feature = "cpucycles")]
#[instrument(fields(cpucycles))]
fn test_function() {
    measure!({})
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    setup_env(args.config_env_file.as_ref());

    if !args.no_banner {
        maybe_print_banner(crate_name!(), crate_version!());
    }

    cfg_if::cfg_if! {
        if #[cfg(feature = "cpucycles")] {
            info!("CPU cycles measurement is ON")
        } else {
            info!("CPU cycles measurement is OFF")
        }
    }

    setup_logging();

    commands::execute(args).await?;

    Ok(())
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

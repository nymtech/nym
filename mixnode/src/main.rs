// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use ::nym_config::defaults::setup_env;
use clap::{crate_name, crate_version, Parser};
use lazy_static::lazy_static;
use log::info;
use nym_bin_common::bin_info;

#[allow(unused_imports)]
use nym_bin_common::logging::{maybe_print_banner, setup_logging};
#[cfg(feature = "cpucycles")]
use nym_bin_common::setup_tracing;
#[cfg(feature = "cpucycles")]
use nym_mixnode_common::measure;
#[cfg(feature = "cpucycles")]
use tracing::instrument;

mod commands;
mod config;
pub(crate) mod error;
mod node;

lazy_static! {
    pub static ref PRETTY_BUILD_INFORMATION: String = bin_info!().pretty_print();
}

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    &PRETTY_BUILD_INFORMATION
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
            setup_tracing!("mixnode");
            info!("CPU cycles measurement is ON")
        } else {
            setup_logging();
            info!("CPU cycles measurement is OFF")
        }
    }

    commands::execute(args).await?;

    cfg_if::cfg_if! {
    if #[cfg(feature = "cpucycles")] {
        opentelemetry::global::shutdown_tracer_provider();
    }}

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

// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate rocket;

use ::nym_config::defaults::setup_env;
use clap::{crate_name, crate_version, Parser};
use lazy_static::lazy_static;
use nym_bin_common::build_information::BinaryBuildInformation;
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
mod node;

lazy_static! {
    pub static ref PRETTY_BUILD_INFORMATION: String =
        BinaryBuildInformation::new(env!("CARGO_PKG_VERSION")).pretty_print();
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

    #[clap(subcommand)]
    command: commands::Commands,
}

#[cfg(feature = "cpucycles")]
#[instrument(fields(cpucycles))]
fn test_function() {
    measure!({})
}

#[tokio::main]
async fn main() {
    cfg_if::cfg_if! {
        if #[cfg(feature = "cpucycles")] {
            let home_dir = dirs::home_dir().expect("Could not get $HOME");
            let logs_dir = home_dir.join(".nym").join("logs");
            let logs_dir_str = logs_dir.to_str().expect("Could not construct logs path");
            setup_tracing!(logs_dir_str);
            info!("CPU cycles measurement is ON")
        } else {
            setup_logging();
            info!("CPU cycles measurement is OFF")
        }
    }

    maybe_print_banner(crate_name!(), crate_version!());

    let args = Cli::parse();
    setup_env(args.config_env_file.as_ref());
    commands::execute(args).await;

    cfg_if::cfg_if! {
    if #[cfg(feature = "cpucycles")] {
        opentelemetry::global::shutdown_tracer_provider();
    }}
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

// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use build_information::BinaryBuildInformation;
use clap::{crate_version, Parser};
use lazy_static::lazy_static;
use logging::setup_logging;
use network_defaults::setup_env;

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
    /// Path pointing to an env file that configures the gateway.
    #[clap(short, long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    #[clap(subcommand)]
    command: commands::Commands,
}

#[tokio::main]
async fn main() {
    setup_logging();
    println!("{}", banner());

    let args = Cli::parse();
    setup_env(args.config_env_file.as_ref());
    commands::execute(args).await;
}

fn banner() -> String {
    format!(
        r#"

      _ __  _   _ _ __ ___
     | '_ \| | | | '_ \ _ \
     | | | | |_| | | | | | |
     |_| |_|\__, |_| |_| |_|
            |___/

             (gateway - version {:})

    "#,
        crate_version!()
    )
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

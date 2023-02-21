// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate rocket;

use ::config::defaults::setup_env;
use build_information::BinaryBuildInformation;
use clap::{crate_name, crate_version, Parser, ValueEnum};
use lazy_static::lazy_static;
use nym_bin_common::logging::setup_logging;

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

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Json,
    Text,
}

impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::Text
    }
}

#[derive(Parser)]
#[clap(author = "Nymtech", version, about, long_version = pretty_build_info_static())]
struct Cli {
    /// Path pointing to an env file that configures the mixnode.
    #[clap(short, long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    #[clap(short, long)]
    pub(crate) output: Option<OutputFormat>,

    #[clap(subcommand)]
    command: commands::Commands,
}

impl Cli {
    fn output(&self) -> OutputFormat {
        if let Some(ref output) = self.output {
            output.clone()
        } else {
            OutputFormat::default()
        }
    }
}

#[tokio::main]
async fn main() {
    setup_logging();
    if atty::is(atty::Stream::Stdout) {
        println!("{}", logging::banner(crate_name!(), crate_version!()));
    }

    let args = Cli::parse();
    setup_env(args.config_env_file.as_ref());
    commands::execute(args).await;
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

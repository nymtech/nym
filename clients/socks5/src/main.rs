// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::error::Error;

use clap::{crate_name, crate_version, Parser};
use nym_bin_common::logging::{banner, setup_logging};
use nym_network_defaults::setup_env;

pub mod client;
mod commands;
pub mod error;
pub mod socks;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    setup_logging();
    println!("{}", banner(crate_name!(), crate_version!()));

    let args = commands::Cli::parse();
    setup_env(args.config_env_file.as_ref());
    commands::execute(&args).await
}

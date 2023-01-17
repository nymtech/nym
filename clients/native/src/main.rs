// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::error::Error;

use clap::{crate_name, crate_version, Parser};
use logging::setup_logging;
use network_defaults::setup_env;

pub mod client;
pub mod commands;
pub mod error;
pub mod websocket;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    setup_logging();
    println!("{}", logging::banner(crate_name!(), crate_version!()));

    let args = commands::Cli::parse();
    setup_env(args.config_env_file.as_ref());
    commands::execute(&args).await
}

// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::error::Error;

use clap::{crate_name, crate_version, Parser};
use nym_bin_common::logging::{maybe_print_banner, setup_logging};
use nym_network_defaults::setup_env;

pub mod client;
pub mod commands;
pub mod error;
pub mod websocket;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args = commands::Cli::parse();
    setup_env(args.config_env_file.as_ref());

    if !args.no_banner {
        maybe_print_banner(crate_name!(), crate_version!());
    }
    setup_logging();

    if let Err(err) = commands::execute(args).await {
        log::error!("{err}");
        println!("An error occurred: {err}");
        std::process::exit(1);
    }
    Ok(())
}

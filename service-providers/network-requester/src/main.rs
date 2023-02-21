// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{crate_name, crate_version, Parser};
use nym_network_defaults::setup_env;
use nym_bin_common::logging::{banner, setup_logging};

use error::NetworkRequesterError;

mod allowed_hosts;
mod cli;
mod config;
mod core;
mod error;
mod reply;
mod socks5;
mod statistics;

#[tokio::main]
async fn main() -> Result<(), NetworkRequesterError> {
    setup_logging();
    println!("{}", banner(crate_name!(), crate_version!()));

    let args = cli::Cli::parse();
    setup_env(args.config_env_file.as_ref());

    cli::execute(args).await
}

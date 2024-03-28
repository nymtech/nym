// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use clap::{crate_name, crate_version, Parser};
use nym_bin_common::logging::{maybe_print_banner, setup_logging};
use nym_network_defaults::setup_env;

mod cli;
mod config;
mod core;
mod error;
mod reply;
mod request_filter;
mod socks5;
mod statistics;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = cli::Cli::parse();
    setup_env(args.config_env_file.as_ref());

    if !args.no_banner {
        maybe_print_banner(crate_name!(), crate_version!());
    }
    setup_logging();

    cli::execute(args).await?;

    Ok(())
}

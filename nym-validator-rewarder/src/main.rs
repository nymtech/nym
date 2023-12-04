// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::Cli;
use clap::{crate_name, crate_version, Parser};
use nym_bin_common::logging::maybe_print_banner;
use nym_network_defaults::setup_env;

pub mod cli;
pub mod config;
pub mod error;
mod rewarder;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    setup_env(args.config_env_file.as_ref());

    if !args.no_banner {
        maybe_print_banner(crate_name!(), crate_version!());
    }

    args.execute().await?;

    Ok(())
}

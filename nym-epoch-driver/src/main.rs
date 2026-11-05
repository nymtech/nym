// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::Cli;
use clap::{Parser, crate_name, crate_version};
use nym_bin_common::logging::{maybe_print_banner, setup_tracing_logger};
use nym_config::defaults::setup_env;

mod cli;
mod config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    setup_env(args.config_env_file.as_ref());
    setup_tracing_logger();

    if !args.no_banner {
        maybe_print_banner(crate_name!(), crate_version!());
    }

    args.execute().await
}

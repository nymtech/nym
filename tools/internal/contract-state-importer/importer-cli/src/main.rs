// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::commands::Cli;
use clap::Parser;
use nym_bin_common::logging::setup_tracing_logger;
use nym_network_defaults::setup_env;

pub mod commands;
pub mod helpers;
pub mod state;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    setup_env(cli.config_env_file.as_ref());

    setup_tracing_logger();
    cli.execute().await
}

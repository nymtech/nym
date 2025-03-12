// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::commands::Cli;
use clap::Parser;
use nym_bin_common::logging::setup_tracing_logger;
use nym_network_defaults::setup_env;
use tracing::trace;

mod commands;
mod helpers;
mod models;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    trace!("args: {cli:#?}");

    setup_env(cli.config_env_file.as_ref());
    setup_tracing_logger();

    cli.execute().await?;
    Ok(())
}

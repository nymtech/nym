// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]

use crate::commands::Cli;
use clap::Parser;
use nym_bin_common::logging::setup_tracing_logger;

mod commands;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_tracing_logger();
    let cli = Cli::parse();

    cli.execute().await
}

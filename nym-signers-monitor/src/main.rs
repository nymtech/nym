// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::Cli;
use clap::Parser;
use nym_bin_common::bin_info_owned;
use nym_bin_common::logging::setup_no_otel_logger;
use tracing::{info, trace};

mod cli;
mod monitor;
pub(crate) mod test_result;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_no_otel_logger().expect("failed to initialize logging");
    let cli = Cli::parse();
    trace!("args: {cli:#?}");

    let bin_info = bin_info_owned!();
    info!("using the following version: {bin_info}");

    cli.execute().await
}

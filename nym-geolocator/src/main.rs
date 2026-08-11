// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::Args;
use clap::Parser;
use nym_bin_common::bin_info_owned;
use nym_bin_common::logging::setup_tracing_logger;
use nym_network_defaults::setup_env;
use tracing::info;

pub(crate) mod cli;
pub(crate) mod error;
pub(crate) mod geolocator;
pub(crate) mod http;
pub(crate) mod ip_info_lookup;
pub(crate) mod node_scraper;
pub(crate) mod nyx;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_tracing_logger();
    let args = Args::parse();
    setup_env(args.config_env_file.as_ref());

    let bin_info = bin_info_owned!();
    info!("using the following version: {bin_info}");

    Ok(())
}

// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::Cli;
use clap::Parser;
use nym_bin_common::logging::tracing_subscriber::layer::SubscriberExt;
use nym_bin_common::logging::tracing_subscriber::util::SubscriberInitExt;
use nym_bin_common::logging::{
    default_tracing_env_filter, default_tracing_fmt_layer, tracing_subscriber,
};
use nym_network_defaults::setup_env;
use tracing::info;

mod cli;
mod orchestrator;
mod storage;

fn setup_logger() -> anyhow::Result<()> {
    // crates that are more granularly filtered, regardless of default `RUST_LOG` value
    let filter_crates = ["reqwest", "hyper"];

    let mut env_filter = default_tracing_env_filter();
    for crate_name in filter_crates {
        env_filter = env_filter.add_directive(format!("{crate_name}=warn").parse()?);
    }

    tracing_subscriber::registry()
        .with(default_tracing_fmt_layer(std::io::stderr))
        .with(env_filter)
        .init();

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_logger()?;
    let cli = Cli::parse();
    setup_env(cli.config_env_file.as_ref());

    cli.execute().await?;

    info!("network monitor orchestrator is done - quitting");
    Ok(())
}

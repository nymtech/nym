// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]

use crate::cli::Cli;
use clap::{crate_name, crate_version, Parser};
use nym_bin_common::logging::{maybe_print_banner, setup_tracing_logger};
use nym_config::defaults::setup_env;

mod cli;
mod env;
pub(crate) mod node;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var(
        "RUST_LOG",
        "debug,tendermint_rpc=warn,h2=warn,hyper=warn,rustls=warn,reqwest=warn,tungstenite=warn,async_tungstenite=warn",
    );

    let cli = Cli::parse();
    setup_env(cli.config_env_file.as_ref());
    setup_tracing_logger();

    if !cli.no_banner {
        maybe_print_banner(crate_name!(), crate_version!());
    }

    cli.execute().await?;

    Ok(())
}

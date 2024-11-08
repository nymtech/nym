// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]

use crate::{cli::Cli, logging::setup_tracing_logger};
use clap::{crate_name, crate_version, Parser};
use nym_bin_common::logging::maybe_print_banner;
use nym_config::defaults::setup_env;

mod cli;
mod env;
mod logging;
pub(crate) mod node;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // std::env::set_var(
    //     "RUST_LOG",
    //     "trace,handlebars=warn,tendermint_rpc=warn,h2=warn,hyper=warn,rustls=warn,reqwest=warn,tungstenite=warn,async_tungstenite=warn,tokio_util=warn,tokio_tungstenite=warn,tokio-util=warn",
    // );

    let cli = Cli::parse();
    setup_env(cli.config_env_file.as_ref());
    setup_tracing_logger();

    if !cli.no_banner {
        maybe_print_banner(crate_name!(), crate_version!());
    }

    cli.execute().await?;

    Ok(())
}

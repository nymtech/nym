// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

// Allow dead code for not(unix)
#![cfg_attr(not(unix), allow(dead_code))]

use crate::cli::Cli;
use clap::Parser;
use nym_bin_common::logging::setup_tracing_logger;

pub(crate) mod cli;
pub(crate) mod error;
mod helpers;
mod manager;

#[cfg(unix)]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // std::env::set_var(
    //     "RUST_LOG",
    //     "trace,handlebars=warn,tendermint_rpc=warn,h2=warn,hyper=warn,rustls=warn,reqwest=warn,tungstenite=warn,async_tungstenite=warn,tokio_util=warn,tokio_tungstenite=warn,tokio-util=warn",
    // );

    let cli = Cli::parse();
    setup_tracing_logger();

    cli.execute().await?;

    Ok(())
}

#[cfg(not(unix))]
fn main() {
    println!("This binary is only supported on Unix systems");
}

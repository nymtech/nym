// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::todo)]
#![warn(clippy::dbg_macro)]

use crate::cli::Cli;
use clap::{crate_name, crate_version, Parser};
use nym_bin_common::logging::{maybe_print_banner, setup_tracing_logger};
use nym_network_defaults::setup_env;

pub mod cli;
pub mod config;
pub mod error;
pub mod rewarder;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // std::env::set_var(
    //     "RUST_LOG",
    //     "trace,handlebars=warn,tendermint_rpc=warn,h2=warn,hyper=warn,rustls=warn,reqwest=warn,tungstenite=warn,async_tungstenite=warn,tokio_util=warn,tokio_tungstenite=warn,tokio-util=warn",
    // );

    let args = Cli::parse();
    setup_env(args.config_env_file.as_ref());
    setup_tracing_logger();

    if !args.no_banner {
        maybe_print_banner(crate_name!(), crate_version!());
    }

    args.execute().await?;

    Ok(())
}

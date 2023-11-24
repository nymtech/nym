// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// based on https://github.com/ratatui-org/ratatui-async-template template

use crate::app::App;
use crate::cli::Args;
use clap::Parser;

pub mod action;
pub mod app;
pub mod cli;
pub mod components;
pub mod keybindings;
pub mod tui;
pub mod utils;
mod nyxd;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    nym_network_defaults::setup_env(args.config_env_file.as_ref());

    let mut app = App::new(args).await?;
    app.run().await?;
    Ok(())
}

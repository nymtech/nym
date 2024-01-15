// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#![cfg(unix)]
#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::todo)]
#![warn(clippy::dbg_macro)]

use crate::cli::Cli;
use clap::Parser;

pub(crate) mod cli;
pub(crate) mod config;
pub(crate) mod daemon;
pub(crate) mod env;
pub(crate) mod error;
pub(crate) mod helpers;
pub(crate) mod tasks;
pub(crate) mod upgrades;

#[cfg(unix)]
fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    Ok(args.execute()?)
}

#[cfg(not(unix))]
fn main() {
    panic!("nymvisor is not supported on this platform")
}

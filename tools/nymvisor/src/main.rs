// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::todo)]
#![warn(clippy::dbg_macro)]

#[cfg(unix)]
pub(crate) mod cli;

#[cfg(unix)]
pub(crate) mod config;

#[cfg(unix)]
pub(crate) mod daemon;

#[cfg(unix)]
pub(crate) mod env;

#[cfg(unix)]
pub(crate) mod error;

#[cfg(unix)]
pub(crate) mod helpers;

#[cfg(unix)]
pub(crate) mod tasks;

#[cfg(unix)]
pub(crate) mod upgrades;

#[cfg(unix)]
fn main() -> anyhow::Result<()> {
    use clap::Parser;

    let args = crate::cli::Cli::parse();

    Ok(args.execute()?)
}

#[cfg(not(unix))]
fn main() {
    panic!("nymvisor is not supported on this platform")
}

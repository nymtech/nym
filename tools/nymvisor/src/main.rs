// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::Cli;
use clap::Parser;

pub(crate) mod cli;
pub(crate) mod config;
pub(crate) mod daemon;
pub(crate) mod env;
pub(crate) mod error;

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    Ok(args.execute()?)
}

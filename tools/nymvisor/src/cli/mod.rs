// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Parser, Subcommand};
use lazy_static::lazy_static;
use nym_bin_common::bin_info;

lazy_static! {
    pub static ref PRETTY_BUILD_INFORMATION: String = bin_info!().pretty_print();
}

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    &PRETTY_BUILD_INFORMATION
}

#[derive(Parser, Debug)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub(crate) struct Cli {
    // I doubt we're gonna need any global flags here, but I'm going to leave the the option open
    // so that'd be easier to add them later if needed
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    Init,
    Run,
    BuildInfo,
    AddUpgrade,
    Config,
}

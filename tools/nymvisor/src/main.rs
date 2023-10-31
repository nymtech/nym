// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::Cli;
use clap::Parser;

pub(crate) mod cli;

fn main() {
    let args = Cli::parse();
    println!("{args:#?}");
}

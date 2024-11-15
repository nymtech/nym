// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use colored::Color::TrueColor;
use colored::Colorize;
use nym_node::error::NymNodeError;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, hide = true)]
    _args: Vec<String>,
}

pub(crate) async fn execute(_args: Args) -> Result<(), NymNodeError> {
    let orange = TrueColor {
        r: 251,
        g: 110,
        b: 78,
    };

    println!("{}", "** Attention **".color(orange).bold());
    print!("This binary ");
    print!("{}", "DOES NOT".color(orange).bold());
    print!("' support migrating from older mixnode/gateway binaries anymore ");
    println!();
    println!("please use an older version instead before attempting to use this binary again.");
    println!();

    Err(NymNodeError::UnsupportedMigration)
}

// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{App, ArgMatches};

pub fn command_args<'a, 'b>() -> App<'a, 'b> {
    App::new("version").about("Displays version and build information of this binary")
}

pub fn execute(_matches: &ArgMatches) {
    println!(
        "{:<20}{}",
        "Build Timestamp:",
        env!("VERGEN_BUILD_TIMESTAMP")
    );
    println!("{:<20}{}", "Build Version:", env!("VERGEN_BUILD_SEMVER"));
    println!("{:<20}{}", "Commit SHA:", env!("VERGEN_GIT_SHA"));
    println!(
        "{:<20}{}",
        "Commit Date:",
        env!("VERGEN_GIT_COMMIT_TIMESTAMP")
    );
    println!("{:<20}{}", "Commit Branch:", env!("VERGEN_GIT_BRANCH"));
    println!("{:<20}{}", "rustc Version:", env!("VERGEN_RUSTC_SEMVER"));
    println!("{:<20}{}", "rustc Channel:", env!("VERGEN_RUSTC_CHANNEL"));
    println!("{:<20}{}", "cargo Profile:", env!("VERGEN_CARGO_PROFILE"));
}

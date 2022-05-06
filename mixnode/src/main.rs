#[macro_use]
extern crate rocket;

// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use ::config::defaults::DEFAULT_NETWORK;
use clap::{crate_version, Parser};
use lazy_static::lazy_static;

mod commands;
mod config;
mod node;

lazy_static! {
    pub static ref LONG_ABOUT: String = long_version();
}

// Helper for passing LONG_ABOUT to clap
fn long_about() -> &'static str {
    &LONG_ABOUT
}

#[derive(Parser)]
#[clap(author = "Nymtech", version, about, long_about = Some(long_about()))]
struct Cli {
    #[clap(subcommand)]
    command: commands::Commands,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    setup_logging();
    println!("{}", banner());

    let args = Cli::parse();
    commands::execute(args).await;
}

fn banner() -> String {
    format!(
        r#"

      _ __  _   _ _ __ ___
     | '_ \| | | | '_ \ _ \
     | | | | |_| | | | | | |
     |_| |_|\__, |_| |_| |_|
            |___/

             (mixnode - version {:})

    "#,
        crate_version!()
    )
}

fn long_version() -> String {
    format!(
        r#"
{:<20}{}
{:<20}{}
{:<20}{}
{:<20}{}
{:<20}{}
{:<20}{}
{:<20}{}
{:<20}{}
{:<20}{}
"#,
        "Build Timestamp:",
        env!("VERGEN_BUILD_TIMESTAMP"),
        "Build Version:",
        env!("VERGEN_BUILD_SEMVER"),
        "Commit SHA:",
        env!("VERGEN_GIT_SHA"),
        "Commit Date:",
        env!("VERGEN_GIT_COMMIT_TIMESTAMP"),
        "Commit Branch:",
        env!("VERGEN_GIT_BRANCH"),
        "rustc Version:",
        env!("VERGEN_RUSTC_SEMVER"),
        "rustc Channel:",
        env!("VERGEN_RUSTC_CHANNEL"),
        "cargo Profile:",
        env!("VERGEN_CARGO_PROFILE"),
        "Network:",
        DEFAULT_NETWORK
    )
}

fn setup_logging() {
    let mut log_builder = pretty_env_logger::formatted_timed_builder();
    if let Ok(s) = ::std::env::var("RUST_LOG") {
        log_builder.parse_filters(&s);
    } else {
        // default to 'Info'
        log_builder.filter(None, log::LevelFilter::Info);
    }

    log_builder
        .filter_module("hyper", log::LevelFilter::Warn)
        .filter_module("tokio_reactor", log::LevelFilter::Warn)
        .filter_module("reqwest", log::LevelFilter::Warn)
        .filter_module("mio", log::LevelFilter::Warn)
        .filter_module("want", log::LevelFilter::Warn)
        .init();
}

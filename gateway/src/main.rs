// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{crate_version, Parser};
use network_defaults::DEFAULT_NETWORK;
use once_cell::sync::OnceCell;

mod commands;
mod config;
mod node;

static LONG_ABOUT: OnceCell<String> = OnceCell::new();

// Helper for passing LONG_ABOUT to clap
fn long_about() -> &'static str {
    LONG_ABOUT.get().expect("Failed to get long about text")
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
    LONG_ABOUT
        .set(long_version())
        .expect("Failed to set long about text");

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

             (gateway - version {:})

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
        .filter_module("sled", log::LevelFilter::Warn)
        .filter_module("tungstenite", log::LevelFilter::Warn)
        .filter_module("tokio_tungstenite", log::LevelFilter::Warn)
        .init();
}

#[macro_use]
extern crate rocket;

// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use ::config::defaults::setup_env;
use clap::{crate_version, Parser};
use lazy_static::lazy_static;
use logging::setup_logging;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Registry, filter};
use tracing_flame::FlameLayer;

mod commands;
mod config;
mod node;

lazy_static! {
    pub static ref LONG_VERSION: String = long_version();
}

// Helper for passing LONG_VERSION to clap
fn long_version_static() -> &'static str {
    &LONG_VERSION
}

#[derive(Parser)]
#[clap(author = "Nymtech", version, about, long_version = long_version_static())]
struct Cli {
    /// Path pointing to an env file that configures the mixnode.
    #[clap(short, long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    #[clap(subcommand)]
    command: commands::Commands,
}

#[tokio::main]
async fn main() {
    //setup_logging();

    //let tracer = opentelemetry_jaeger::new_agent_pipeline()
    //.with_endpoint("143.42.21.138:6831")
    //.with_service_name("nym_mixnode1")
    //.with_auto_split_batch(true)
    //.install_batch(opentelemetry::runtime::Tokio)
    //.expect("Failed to initialize tracer");

    //let jaeger_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let hyper_filter = filter::filter_fn(|metadata| {!metadata.target().starts_with("hyper")});
    let tokio_filter = filter::filter_fn(|metadata| {!metadata.target().starts_with("tokio")});
    //let (flame_layer, _guard) = FlameLayer::with_file("./tracing.folded").unwrap();

    let subscriber = Registry::default()
        .with(EnvFilter::from_default_env())
        .with(hyper_filter)
        .with(tokio_filter)
        .with(tracing_subscriber::fmt::layer().pretty());
        //.with(flame_layer);
        //.with(jaeger_layer);

    
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global subscriber");

    println!("{}", banner());

    let args = Cli::parse();
    setup_env(args.config_env_file.clone());
    commands::execute(args).await;
    opentelemetry::global::shutdown_tracer_provider();
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
        env!("VERGEN_CARGO_PROFILE")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert();
    }
}

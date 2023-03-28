// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use build_information::BinaryBuildInformation;
use clap::{crate_name, crate_version, Parser, ValueEnum};
use colored::Colorize;
use lazy_static::lazy_static;
use logging::setup_logging;
use network_defaults::setup_env;
use std::error::Error;

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Registry, filter};
use tracing::*;
use tracing_flame::FlameLayer;
use std::time::Duration;
use std::thread::sleep;



mod commands;
mod config;
pub(crate) mod error;
mod node;
pub(crate) mod support;

lazy_static! {
    pub static ref PRETTY_BUILD_INFORMATION: String =
        BinaryBuildInformation::new(env!("CARGO_PKG_VERSION")).pretty_print();
}

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    &PRETTY_BUILD_INFORMATION
}

#[derive(Clone, ValueEnum)]
pub enum OutputFormat {
    Json,
    Text,
}

impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::Text
    }
}

#[derive(Parser)]
#[clap(author = "Nymtech", version, about, long_version = pretty_build_info_static())]
struct Cli {
    /// Path pointing to an env file that configures the gateway.
    #[clap(short, long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    #[clap(short, long)]
    pub(crate) output: Option<OutputFormat>,

    #[clap(subcommand)]
    command: commands::Commands,
}

impl Cli {
    fn output(&self) -> OutputFormat {
        if let Some(ref output) = self.output {
            output.clone()
        } else {
            OutputFormat::default()
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_endpoint("143.42.21.138:6831")
        .with_service_name("nym_gateway")
        .with_auto_split_batch(true)
        .install_batch(opentelemetry::runtime::Tokio)
        .expect("Failed to initialize tracer");

    //let jaeger_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let hyper_filter = filter::filter_fn(|metadata| {!metadata.target().starts_with("hyper")});
    let tokio_filter = filter::filter_fn(|metadata| {!metadata.target().starts_with("tokio")});

    let (flame_layer, _guard) = FlameLayer::with_file("./tracing.folded").unwrap();

    let subscriber = Registry::default()
        .with(EnvFilter::from_default_env())
        .with(hyper_filter)
        .with(tokio_filter)
        .with(tracing_subscriber::fmt::layer().pretty())
        .with(flame_layer);
        //.with(jaeger_layer)


   // tracing::subscriber::set_global_default(subscriber)
    // .expect("Failed to set global subscriber");
    //tracing_subscriber::fmt::init();
    setup_logging();
    if atty::is(atty::Stream::Stdout) {
        println!("{}", logging::banner(crate_name!(), crate_version!()));
    }

    let args = Cli::parse();
    setup_env(args.config_env_file.as_ref());
//.instrument(info_span!("Execute Run"))
    commands::execute(args).await.map_err(|err| {
        if atty::is(atty::Stream::Stdout) {
            let error_message = format!("{err}").red();
            error!("{error_message}");
            error!("Exiting...");
        }
        err
    });
    //sleep(Duration::from_secs(5));
    //opentelemetry::global::shutdown_tracer_provider();
    Ok(())

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

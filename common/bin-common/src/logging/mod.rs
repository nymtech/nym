// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod error;

use error::TracingError;
use serde::{Deserialize, Serialize};
use std::io::IsTerminal;
use tracing_subscriber::{filter::Directive, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Default, Copy, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LoggingSettings {
    // well, we need to implement something here at some point...
}

// don't call init so that we could attach additional layers
pub fn build_tracing_logger() -> impl tracing_subscriber::layer::SubscriberExt {
    use tracing_subscriber::prelude::*;

    tracing_subscriber::registry()
        .with(default_tracing_fmt_layer(std::io::stderr))
        .with(default_tracing_env_filter())
}

pub fn default_tracing_env_filter() -> tracing_subscriber::filter::EnvFilter {
    if ::std::env::var("RUST_LOG").is_ok() {
        tracing_subscriber::filter::EnvFilter::from_default_env()
    } else {
        // if the env value was not found, default to `INFO` level rather than `ERROR`
        tracing_subscriber::filter::EnvFilter::builder()
            .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
            .parse_lossy("")
    }
}

pub fn default_tracing_fmt_layer<S, W>(
    writer: W,
) -> impl tracing_subscriber::Layer<S> + Sync + Send + 'static
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
    W: for<'writer> tracing_subscriber::fmt::MakeWriter<'writer> + Sync + Send + 'static,
{
    tracing_subscriber::fmt::layer()
        .with_writer(writer)
        // Use a more compact, abbreviated log format
        .compact()
        // Display source code file paths
        .with_file(true)
        // Display source code line numbers
        .with_line_number(true)
        // Don't display the event's target (module path)
        .with_target(false)
}

/// Creates a tracing filter that sets more granular log levels for specific crates.
/// This allows for finer control over logging verbosity.
pub(crate) fn granual_filtered_env() -> Result<tracing_subscriber::filter::EnvFilter, TracingError>
{
    fn directive_checked(directive: impl Into<String>) -> Result<Directive, TracingError> {
        directive.into().parse().map_err(From::from)
    }

    let mut filter = default_tracing_env_filter();

    // these crates are more granularly filtered
    let filter_crates = ["defguard_wireguard_rs"];
    for crate_name in filter_crates {
        filter = filter.add_directive(directive_checked(format!("{crate_name}=warn"))?);
    }
    Ok(filter)
}

pub fn setup_no_otel_logger() -> Result<(), TracingError> {
    // Only set up if not already initialized
    if tracing::dispatcher::has_been_set() {
        // It shouldn't be - this is really checking that it is torn down between async command executions
        return Err(TracingError::TracingLoggerAlreadyInitialised);
    }

    let registry = tracing_subscriber::registry()
        .with(default_tracing_fmt_layer(std::io::stderr))
        .with(granual_filtered_env()?);

    registry
        .try_init()
        .map_err(|e| TracingError::TracingTryInitError(e))?;

    Ok(())
}

// TODO: This has to be a macro, running it as a function does not work for the file_appender for some reason
#[macro_export]
macro_rules! setup_tracing {
    ($service_name: expr) => {
        setup_no_otel_logger()
    };
}

pub fn banner(crate_name: &str, crate_version: &str) -> String {
    format!(
        r#"

      _ __  _   _ _ __ ___
     | '_ \| | | | '_ \ _ \
     | | | | |_| | | | | | |
     |_| |_|\__, |_| |_| |_|
            |___/

             ({crate_name} - version {crate_version})

    "#
    )
}

pub fn maybe_print_banner(crate_name: &str, crate_version: &str) {
    if std::io::stdout().is_terminal() {
        println!("{}", banner(crate_name, crate_version))
    }
}

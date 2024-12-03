// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::io::IsTerminal;

#[cfg(feature = "tracing")]
pub use opentelemetry;
#[cfg(feature = "tracing")]
pub use opentelemetry_jaeger;
#[cfg(feature = "tracing")]
pub use tracing_opentelemetry;
#[cfg(feature = "tracing")]
pub use tracing_subscriber;
#[cfg(feature = "tracing")]
pub use tracing_tree;

#[cfg(target_os = "macos")]
const MACOS_LOG_FILEPATH: &str = "/var/log/nym-vpnd/daemon.log";
#[cfg(target_os = "ios")]
const IOS_LOG_FILEPATH_VAR: &str = "IOS_LOG_FILEPATH";

#[derive(Debug, Default, Copy, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LoggingSettings {
    // well, we need to implement something here at some point...
}

/// Enables and configures logging using the `log` and `pretty_env_logger` libraries.
///
/// On call this subscriber attempts to parse filter level from the `"RUST_LOG"` environment variable.
/// If that is not set it defaults to `INFO` level.
///
/// As logs are not available to iOS or MacOS apps through the console, logs can be written to
/// file for handling. On iOS if a path is provided in the `"IOS__LOG_FILEPATH"` variable this
/// function will attempt to open that file and use it as the logging sink. On MacOS logs are
/// written to the static `"/var/log/nym-vpnd/daemon.log"`. If we are unable to open the
/// log filepath for either iOS or MacOS we default to writing to the default (console) output.
// I'd argue we should start transitioning from `log` to `tracing`
pub fn setup_logging() {
    let mut log_builder = pretty_env_logger::formatted_timed_builder();
    if let Ok(s) = ::std::env::var("RUST_LOG") {
        log_builder.parse_filters(&s);
    } else {
        // default to 'Info'
        log_builder.filter(None, log::LevelFilter::Info);
    }

    #[cfg(target_os = "macos")]
    if let Ok(f) = ::std::fs::File::create(MACOS_LOG_FILEPATH) {
        log_builder.target(env_logger::fmt::Target::Pipe(Box::new(f)));
    }

    #[cfg(target_os = "ios")]
    if let Ok(logfile_path) = ::std::env::var(IOS_LOG_FILEPATH_VAR) {
        if let Ok(f) = File::create(logfile_path) {
            log_builder.target(env_logger::fmt::Target::Pipe(Box::new(f)));
        }
    }

    log_builder
        .filter_module("hyper", log::LevelFilter::Warn)
        .filter_module("tokio_reactor", log::LevelFilter::Warn)
        .filter_module("reqwest", log::LevelFilter::Warn)
        .filter_module("mio", log::LevelFilter::Warn)
        .filter_module("want", log::LevelFilter::Warn)
        .filter_module("tungstenite", log::LevelFilter::Warn)
        .filter_module("tokio_tungstenite", log::LevelFilter::Warn)
        .filter_module("handlebars", log::LevelFilter::Warn)
        .filter_module("sled", log::LevelFilter::Warn)
        .init();
}

/// Enables and configures logging using the `tracing` and `tracing-subscriber` libraries.
///
/// On call this subscriber attempts to parse filter level from the `"RUST_LOG"` environment variable.
/// If that is not set it defaults to `INFO` level.
///
/// As logs are not available to iOS or MacOS apps through the console, logs can be written to
/// file for handling. On iOS if a path is provided in the `"IOS__LOG_FILEPATH"` variable this
/// function will attempt to open that file and use it as the logging sink. On MacOS logs are
/// written to the static `"/var/log/nym-vpnd/daemon.log"`. If we are unable to open the
/// log filepath for either iOS or MacOS we default to writing to the default (console) output.
#[cfg(feature = "basic_tracing")]
pub fn setup_tracing_logger() {
    let log_builder = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        // Use a more compact, abbreviated log format
        .compact()
        // Display source code file paths
        .with_file(true)
        // Display source code line numbers
        .with_line_number(true)
        // Don't display the event's target (module path)
        .with_target(false);

    #[cfg(target_os = "macos")]
    // Attempt to set the log sink to a file on macos, if we fail to open the file fallback to default (console)
    if let Ok(f) = ::std::fs::File::create(MACOS_LOG_FILEPATH) {
        log_builder.with_writer(Mutex::new(f));
    }

    #[cfg(target_os = "ios")]
    // Attempt to set the log sink to a file on ios, if the env var is not set or we fail to open the file
    //  fallback to default (console)
    if let Ok(logfile_path) = ::std::env::var(IOS_LOG_FILEPATH_VAR) {
        if let Ok(f) = ::std::fs::File::create(logfile_path) {
            log_builder.with_writer(Mutex::new(f));
        }
    }

    if ::std::env::var("RUST_LOG").is_ok() {
        log_builder
            .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
            .init()
    } else {
        // default to 'Info
        log_builder
            .with_max_level(tracing_subscriber::filter::LevelFilter::INFO)
            .init()
    }
}

// TODO: This has to be a macro, running it as a function does not work for the file_appender for some reason
#[cfg(feature = "tracing")]
#[macro_export]
macro_rules! setup_tracing {
    ($service_name: expr) => {
        use nym_bin_common::logging::tracing_subscriber::layer::SubscriberExt;
        use nym_bin_common::logging::tracing_subscriber::util::SubscriberInitExt;

        let registry = nym_bin_common::logging::tracing_subscriber::Registry::default()
            .with(nym_bin_common::logging::tracing_subscriber::EnvFilter::from_default_env())
            .with(
                nym_bin_common::logging::tracing_tree::HierarchicalLayer::new(4)
                    .with_targets(true)
                    .with_bracketed_fields(true),
            );

        let tracer = nym_bin_common::logging::opentelemetry_jaeger::new_collector_pipeline()
            .with_endpoint("http://44.199.230.10:14268/api/traces")
            .with_service_name($service_name)
            .with_isahc()
            .with_trace_config(
                nym_bin_common::logging::opentelemetry::sdk::trace::config().with_sampler(
                    nym_bin_common::logging::opentelemetry::sdk::trace::Sampler::TraceIdRatioBased(
                        0.1,
                    ),
                ),
            )
            .install_batch(nym_bin_common::logging::opentelemetry::runtime::Tokio)
            .expect("Could not init tracer");

        let telemetry = nym_bin_common::logging::tracing_opentelemetry::layer().with_tracer(tracer);

        registry.with(telemetry).init();
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

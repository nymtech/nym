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

#[derive(Debug, Default, Copy, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LoggingSettings {
    // well, we need to implement something here at some point...
}

// I'd argue we should start transitioning from `log` to `tracing`
pub fn setup_logging() {
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
        .filter_module("tungstenite", log::LevelFilter::Warn)
        .filter_module("tokio_tungstenite", log::LevelFilter::Warn)
        .filter_module("handlebars", log::LevelFilter::Warn)
        .filter_module("sled", log::LevelFilter::Warn)
        .init();
}

#[cfg(feature = "basic_tracing")]
pub fn setup_tracing_logger() {
    let log_builder = tracing_subscriber::fmt()
        // Use a more compact, abbreviated log format
        .compact()
        // Display source code file paths
        .with_file(true)
        // Display source code line numbers
        .with_line_number(true)
        // Don't display the event's target (module path)
        .with_target(false);

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

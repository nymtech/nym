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

// don't call init so that we could attach additional layers
#[cfg(feature = "basic_tracing")]
pub fn build_tracing_logger() -> impl tracing_subscriber::layer::SubscriberExt {
    use tracing_subscriber::prelude::*;

    tracing_subscriber::registry()
        .with(default_tracing_fmt_layer(std::io::stderr))
        .with(default_tracing_env_filter())
}

#[cfg(feature = "basic_tracing")]
pub fn default_tracing_env_filter() -> tracing_subscriber::filter::EnvFilter {
    if ::std::env::var("RUST_LOG").is_ok() {
        tracing_subscriber::filter::EnvFilter::from_default_env()
    } else {
        // if the env value was not found, default to `INFO` level rather than `ERROR`
        tracing_subscriber::filter::EnvFilter::builder()
            .with_default_directive(tracing_subscriber::filter::LevelFilter::DEBUG.into())
            .parse_lossy("")
    }
}

#[cfg(feature = "basic_tracing")]
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

#[cfg(feature = "basic_tracing")]
pub fn setup_tracing_logger() {
    use tracing_subscriber::util::SubscriberInitExt;
    build_tracing_logger().init()
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

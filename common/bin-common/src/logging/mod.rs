// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::io::IsTerminal;

// Re-export tracing_subscriber for consumers that need to compose layers
#[cfg(feature = "basic_tracing")]
pub use tracing_subscriber;

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
            .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
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

/// Initialize an OpenTelemetry tracing layer that exports spans via OTLP/gRPC.
///
/// This produces a layer compatible with `tracing_subscriber::registry()` that
/// sends traces to any OTLP-compatible collector (SigNoz, Grafana Tempo, etc).
///
/// Returns both the tracing layer and the [`SdkTracerProvider`] so the caller
/// can invoke [`SdkTracerProvider::shutdown`] for graceful flush on exit.
///
/// # Arguments
/// * `service_name` - The service name reported to the collector (e.g. "nym-node")
/// * `endpoint` - The OTLP/gRPC collector endpoint (e.g. "http://localhost:4317"
///   or "https://ingest.eu.signoz.cloud:443" for SigNoz Cloud)
/// * `ingestion_key` - Optional SigNoz Cloud ingestion key. When provided, it is
///   sent as the `signoz-ingestion-key` gRPC metadata header on every export.
#[cfg(feature = "otel-otlp")]
pub fn init_otel_layer<S>(
    service_name: &str,
    endpoint: &str,
    ingestion_key: Option<&str>,
) -> Result<
    (
        tracing_opentelemetry::OpenTelemetryLayer<S, opentelemetry_sdk::trace::SdkTracer>,
        opentelemetry_sdk::trace::SdkTracerProvider,
    ),
    Box<dyn std::error::Error + Send + Sync>,
>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    use opentelemetry::trace::TracerProvider as _;
    use opentelemetry_otlp::WithExportConfig;

    let mut builder = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint);

    if let Some(key) = ingestion_key {
        use opentelemetry_otlp::WithTonicConfig;
        let mut metadata = tonic::metadata::MetadataMap::new();
        metadata.insert(
            "signoz-ingestion-key",
            key.parse().map_err(|_| "invalid ingestion key value")?,
        );
        builder = builder.with_metadata(metadata);
    }

    let exporter = builder.build()?;

    let tracer_provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(
            opentelemetry_sdk::Resource::builder()
                .with_service_name(service_name.to_owned())
                .build(),
        )
        .build();

    opentelemetry::global::set_tracer_provider(tracer_provider.clone());
    let tracer = tracer_provider.tracer(service_name.to_owned());

    Ok((
        tracing_opentelemetry::layer().with_tracer(tracer),
        tracer_provider,
    ))
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

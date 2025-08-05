// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_bin_common::logging::{default_tracing_env_filter, default_tracing_fmt_layer};
use tracing::{info, warn};
use tracing_subscriber::filter::Directive;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, Layer};

// Signoz
use opentelemetry::trace::TracerProvider;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::tonic_types::metadata::MetadataMap;
use opentelemetry_otlp::tonic_types::transport::ClientTlsConfig;
use opentelemetry_otlp::{WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::metrics::{MeterProviderBuilder, PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::trace::{RandomIdGenerator, SdkTracerProvider};
use opentelemetry_sdk::{trace::Sampler, Resource};
use opentelemetry_semantic_conventions::resource::{DEPLOYMENT_ENVIRONMENT_NAME, SERVICE_VERSION};
use opentelemetry_semantic_conventions::SCHEMA_URL;
use tracing_core::Level;
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::fmt::format::FmtSpan;

pub(crate) fn granual_filtered_env() -> anyhow::Result<tracing_subscriber::filter::EnvFilter> {
    fn directive_checked(directive: impl Into<String>) -> anyhow::Result<Directive> {
        directive.into().parse().map_err(From::from)
    }

    let mut filter = default_tracing_env_filter();

    let filter_crates = ["defguard_wireguard_rs"];
    for crate_name in filter_crates {
        filter = filter.add_directive(directive_checked(format!("{crate_name}=warn"))?);
    }
    Ok(filter)
}

pub(crate) fn build_tracing_logger() -> anyhow::Result<impl SubscriberExt> {
    let key = std::env::var("SIGNOZ_INGESTION_KEY").expect("SIGNOZ_INGESTION_KEY not set");
    println!("SIGNOZ_INGESTION_KEY = {}", key);
    let mut metadata = MetadataMap::new();
    metadata.insert(
        "signoz-ingestion-key",
        key.parse().expect("Could not parse key"),
    );

    let tracer_provider = init_tracer_provider(metadata)?;
    let meter_provider = init_meter_provider()?;
    let tracer = tracer_provider.tracer("tracing-otel-subscriber");

    let fmt_layer = fmt::layer()
        .json()
        .with_writer(std::io::stderr)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_span_list(false)
        .with_current_span(true);

    let registry = tracing_subscriber::registry()
        .with(fmt_layer)
        .with(granual_filtered_env()?)
        .with(tracing_subscriber::filter::LevelFilter::from_level(
            Level::DEBUG,
        ))
        .with(MetricsLayer::new(meter_provider))
        .with(OpenTelemetryLayer::new(tracer));

    Ok(registry)
}

pub(crate) fn setup_tracing_logger() -> anyhow::Result<()> {
    build_tracing_logger()?.init();
    Ok(())
}

// This is called outside of the async context where we can't use OTEL
pub(crate) fn setup_no_otel_logger() -> anyhow::Result<()> {
    // Only set up if not already initialized
    if tracing::dispatcher::has_been_set() {
        // It shouldn't be - this is really checking that it is torn down between async command executions
        return Err(anyhow::anyhow!("Tracing logger already initialised"));
    }

    let registry = tracing_subscriber::registry()
        .with(default_tracing_fmt_layer(std::io::stderr))
        .with(granual_filtered_env()?)
        .with(tracing_subscriber::filter::LevelFilter::from_level(
            Level::INFO,
        ));

    registry
        .try_init()
        .map_err(|e| anyhow::anyhow!("Failed to set tracing subscriber: {}", e))?;

    Ok(())
}

// Signoz/OTEL
fn resource() -> Resource {
    Resource::builder()
        .with_service_name("nym-node")
        .with_schema_url(
            [
                KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
                KeyValue::new(DEPLOYMENT_ENVIRONMENT_NAME, "develop"),
            ],
            SCHEMA_URL,
        )
        .build()
}

fn init_tracer_provider(metadata: MetadataMap) -> anyhow::Result<SdkTracerProvider> {
    let endpoint = std::env::var("SIGNOZ_ENDPOINT").expect("SIGNOZ_ENDPOINT not set");
    info!("SIGNOZ_ENDPOINT = {}", endpoint);

    let mut exporter_builder = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_metadata(metadata)
        .with_endpoint(&endpoint);

    if endpoint.starts_with("https://") {
        exporter_builder =
            exporter_builder.with_tls_config(ClientTlsConfig::new().with_enabled_roots());
    }

    let exporter = exporter_builder.build()?;

    let tracer = SdkTracerProvider::builder()
        .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
            1.0,
        ))))
        .with_id_generator(RandomIdGenerator::default())
        .with_resource(resource())
        .with_batch_exporter(exporter)
        .build();

    Ok(tracer)
}

fn init_meter_provider() -> anyhow::Result<SdkMeterProvider> {
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_temporality(opentelemetry_sdk::metrics::Temporality::default())
        .build()?;

    let reader = PeriodicReader::builder(exporter)
        .with_interval(std::time::Duration::from_secs(30))
        .build();

    let stdout_reader =
        PeriodicReader::builder(opentelemetry_stdout::MetricExporter::default()).build();

    let meter_provider = MeterProviderBuilder::default()
        .with_resource(resource())
        .with_reader(reader)
        .with_reader(stdout_reader)
        .build();

    global::set_meter_provider(meter_provider.clone());

    Ok(meter_provider)
}

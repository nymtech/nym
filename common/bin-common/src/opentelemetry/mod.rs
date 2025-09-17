pub mod context;
mod trace_id_format;

use tracing::{info, Level};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::fmt;
use crate::logging::{
    default_tracing_fmt_layer, 
    error::TracingError,
    granual_filtered_env,
}; 
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
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::fmt::format::FmtSpan;

/// Sets up a tracing subscriber with an OpenTelemetry layer and a metrics layer.
/// The OpenTelemetry exporter is configured to send data to the endpoint specified in the
/// `SIGNOZ_ENDPOINT` environment variable, using the ingestion key specified in the
/// `SIGNOZ_INGESTION_KEY` environment variable.
pub fn setup_tracing_logger(service_name: String) -> Result<(), TracingError> {
    if tracing::dispatcher::has_been_set() {
        // It shouldn't be - this is really checking that it is torn down between async command executions
        return Err(TracingError::TracingLoggerAlreadyInitialised);
    }

    let key =
        std::env::var("SIGNOZ_INGESTION_KEY".to_string()).expect("SIGNOZ_INGESTION_KEY not set");
    let mut metadata = MetadataMap::new();
    metadata.insert(
        "signoz-ingestion-key",
        key.parse().expect("Could not parse signoz ingestion key"),
    );

    let tracer_provider = init_tracer_provider(metadata.clone(), service_name.clone())?;
    let meter_provider = init_meter_provider(metadata.clone(), service_name.clone())?;
    let tracer = tracer_provider.tracer("tracing-otel-subscriber");
    let fmt_layer = fmt::layer()
        .json()
        .with_writer(std::io::stderr)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_span_list(false)
        .with_current_span(true)
        .event_format(trace_id_format::TraceIdFormat);

    cfg_if::cfg_if! {if #[cfg(feature = "tokio-console")] {
        // instrument tokio console subscriber needs RUSTFLAGS="--cfg tokio_unstable" at build time
        let console_layer = console_subscriber::spawn();

        tracing_subscriber::registry()
            .with(console_layer)
            .with(fmt_layer)
            .with(granual_filtered_env()?)
            .with(tracing_subscriber::filter::LevelFilter::from_level(Level::DEBUG))
            .with(MetricsLayer::new(meter_provider))
            .with(OpenTelemetryLayer::new(tracer))
            .try_init()
            .map_err(|e| TracingError::TracingTryInitError(e))?;
    } else {
        tracing_subscriber::registry()
            .with(fmt_layer)
            .with(granual_filtered_env()?)
            .with(tracing_subscriber::filter::LevelFilter::from_level(Level::DEBUG))
            .with(MetricsLayer::new(meter_provider))
            .with(OpenTelemetryLayer::new(tracer))
            .try_init()
            .map_err(|e| TracingError::TracingTryInitError(e))?;
    }}
    
    Ok(())
}

/// Sets up a tracing subscriber without an OpenTelemetry layer.
/// Since Opentelemetry should remain optional, this function provides a local logger setup
/// that can be used when OTEL is not desired.
pub fn setup_no_otel_logger() -> Result<(), TracingError> {
    // Only set up if not already initialized
    if tracing::dispatcher::has_been_set() {
        // It shouldn't be - this is really checking that it is torn down between async command executions
        return Err(TracingError::TracingLoggerAlreadyInitialised);
    }

    let registry = tracing_subscriber::registry()
        .with(default_tracing_fmt_layer(std::io::stderr))
        .with(granual_filtered_env()?)
        .with(tracing_subscriber::filter::LevelFilter::from_level(
            Level::INFO,
        ));

    registry
        .try_init()
        .map_err(|e| TracingError::TracingTryInitError(e))?;

    Ok(())
}

/// Creates a resource with the given service name and additional attributes.
/// The resource includes the service name, service version, and deployment environment.
fn resource(service_name: String) -> Resource {
    Resource::builder()
        .with_service_name(service_name)
        .with_schema_url(
            [
                KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
                KeyValue::new(DEPLOYMENT_ENVIRONMENT_NAME, "develop"),
            ],
            SCHEMA_URL,
        )
        .build()
}

/// Initializes and returns a tracer provider configured to export spans to the endpoint
/// specified in the `SIGNOZ_ENDPOINT` environment variable. The tracer provider is set as
/// the global tracer provider.
fn init_tracer_provider(metadata: MetadataMap, service_name: String) -> Result<SdkTracerProvider, TracingError> {
    let endpoint = std::env::var("SIGNOZ_ENDPOINT".to_string()).expect("SIGNOZ_ENDPOINT not set");
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
        .with_resource(resource(service_name))
        .with_batch_exporter(exporter)
        .build();

    global::set_tracer_provider(tracer.clone());
    Ok(tracer)
}

/// Initializes and returns a meter provider configured to export metrics to the endpoint
/// specified in the `SIGNOZ_ENDPOINT` environment variable. The meter provider is set as
/// the global meter provider.
fn init_meter_provider(metadata: MetadataMap, service_name: String) -> Result<SdkMeterProvider, TracingError> {
    let endpoint = std::env::var("SIGNOZ_ENDPOINT".to_string()).expect("SIGNOZ_ENDPOINT not set");

    let mut exporter_builder = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_metadata(metadata)
        .with_endpoint(&endpoint)
        .with_temporality(opentelemetry_sdk::metrics::Temporality::default());

    if endpoint.starts_with("https://") {
        exporter_builder = exporter_builder.with_tls_config(ClientTlsConfig::new().with_enabled_roots());
    }

    let exporter = exporter_builder.build()?;

    let reader = PeriodicReader::builder(exporter)
        .with_interval(std::time::Duration::from_secs(30))
        .build();

    let stdout_reader =
        PeriodicReader::builder(opentelemetry_stdout::MetricExporter::default()).build();

    let meter_provider = MeterProviderBuilder::default()
        .with_resource(resource(service_name))
        .with_reader(reader)
        .with_reader(stdout_reader)
        .build();

    global::set_meter_provider(meter_provider.clone());

    Ok(meter_provider)
}

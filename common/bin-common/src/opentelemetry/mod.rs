pub mod context;
pub mod compact_id_generator;
mod trace_id_format;

use tracing::{info, Level};
use tracing_subscriber::filter::Directive;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::fmt;

use crate::logging::default_tracing_env_filter;
use crate::logging::error::TracingError;
use crate::opentelemetry::compact_id_generator::Compact13BytesIdGenerator;
use opentelemetry::trace::TracerProvider;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::tonic_types::metadata::MetadataMap;
use opentelemetry_otlp::tonic_types::transport::ClientTlsConfig;
use opentelemetry_otlp::{WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::metrics::{MeterProviderBuilder, PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::{trace::Sampler, Resource};
use opentelemetry_semantic_conventions::resource::{DEPLOYMENT_ENVIRONMENT_NAME, SERVICE_VERSION};
use opentelemetry_semantic_conventions::SCHEMA_URL;
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::fmt::format::FmtSpan;

pub struct TracerProviderGuard(Option<SdkTracerProvider>);

impl Drop for TracerProviderGuard {
    fn drop(&mut self) {
        if let Some(tracer_provider) = self.0.take() {
            // Ensure all spans are flushed before exit
            if let Err(e) = tracer_provider.shutdown() {
                eprintln!("Error shutting down tracer provider: {:?}", e);
            }
        }
    }
}

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

pub fn setup_tracing_logger(service_name: String) -> Result<TracerProviderGuard, TracingError> {
    if tracing::dispatcher::has_been_set() {
        // It shouldn't be - this is really checking that it is torn down between async command executions
        return Err(TracingError::TracingLoggerAlreadyInitialised);
    }

    // define ingestion points
    let endpoint = std::env::var("SIGNOZ_ENDPOINT").expect("SIGNOZ_ENDPOINT not set");
    let key = std::env::var("SIGNOZ_INGESTION_KEY").expect("SIGNOZ_INGESTION_KEY not set");
    let mut metadata = MetadataMap::new();
    metadata.insert(
        "signoz-ingestion-key",
        key.parse().expect("Could not parse signoz ingestion key"),
    );

    // Build resources
    let resource = build_resource(&service_name);

    // Initialize tracer and meter providers
    let tracer_provider = init_tracer_provider(&endpoint, metadata.clone(), resource.clone())?;
    let meter_provider = init_meter_provider(&endpoint, metadata.clone(), resource.clone())?;

    // Bridge tracing and opentelemetry
    let tracer = tracer_provider.tracer("otel-subscriber");
    let fmt_layer = fmt::layer()
        .json()
        .with_writer(std::io::stderr)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_span_list(false)
        .with_current_span(true)
        .event_format(trace_id_format::TraceIdFormat);

    let registry = tracing_subscriber::registry()
        .with(fmt_layer)
        .with(granual_filtered_env()?)
        .with(tracing_subscriber::filter::LevelFilter::from_level(Level::INFO))
        .with(MetricsLayer::new(meter_provider.clone()))
        .with(OpenTelemetryLayer::new(tracer));

    registry.try_init().map_err(TracingError::TracingTryInitError)?;

    global::set_tracer_provider(tracer_provider.clone());
    global::set_meter_provider(meter_provider.clone());

    info!("Tracing initialized with service name: {}", service_name);

    Ok(TracerProviderGuard(Some(tracer_provider)))
}

fn build_resource(service_name: &str) -> Resource {
    Resource::builder()
        .with_service_name(service_name.to_string())
        .with_schema_url(
            [
                KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
                KeyValue::new(DEPLOYMENT_ENVIRONMENT_NAME, "develop"),
            ],
            SCHEMA_URL,
        )
        .build()
}

fn init_tracer_provider(
    endpoint: &str,
    metadata: MetadataMap,
    resource: Resource,
) -> Result<SdkTracerProvider, TracingError> {
    let mut exporter_builder = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_metadata(metadata)
        .with_endpoint(endpoint);

    if endpoint.starts_with("https://") {
        exporter_builder =
            exporter_builder.with_tls_config(ClientTlsConfig::new().with_enabled_roots());
    }

    let exporter = exporter_builder.build()?;

    let tracer = SdkTracerProvider::builder()
        .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
            1.0,
        ))))
        .with_id_generator(Compact13BytesIdGenerator)
        .with_resource(resource)
        .with_batch_exporter(exporter)
        .build();

    global::set_tracer_provider(tracer.clone());
    Ok(tracer)
}

fn init_meter_provider(
    endpoint: &str,
    metadata: MetadataMap,
    resource: Resource,
) -> Result<SdkMeterProvider, TracingError> {
    let mut exporter_builder = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_metadata(metadata)
        .with_endpoint(endpoint)
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
        .with_resource(resource)
        .with_reader(reader)
        .with_reader(stdout_reader)
        .build();

    global::set_meter_provider(meter_provider.clone());

    Ok(meter_provider)
}

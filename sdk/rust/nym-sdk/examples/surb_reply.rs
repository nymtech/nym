use dotenvy::dotenv;
use nym_sdk::mixnet::{
    AnonymousSenderTag, MixnetClientBuilder, MixnetMessageSender, ReconstructedMessage,
    StoragePaths,
};
use opentelemetry::trace::TraceContextExt;
use opentelemetry::trace::Tracer;
use opentelemetry::trace::TracerProvider;
use opentelemetry::Context;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::tonic_types::metadata::MetadataMap;
use opentelemetry_otlp::tonic_types::transport::ClientTlsConfig;
use opentelemetry_otlp::{WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::metrics::{MeterProviderBuilder, PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::trace::IdGenerator;
use opentelemetry_sdk::trace::{RandomIdGenerator, SdkTracerProvider};
use opentelemetry_sdk::{trace::Sampler, Resource};
use opentelemetry_semantic_conventions::{
    attribute::{DEPLOYMENT_ENVIRONMENT_NAME, SERVICE_VERSION},
    SCHEMA_URL,
};
use std::path::PathBuf;
use tempfile::TempDir;
use tracing::info_span;
use tracing::instrument;
use tracing::{info, warn};
use tracing_core::Level;
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn resource() -> Resource {
    Resource::builder()
        .with_service_name("sdk-example-surb-reply")
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
    println!("SIGNOZ_ENDPOINT = {}", endpoint);

    // Configure OTLP exporter with metadata
    let mut exporter_builder = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_metadata(metadata)
        .with_endpoint(&endpoint);

    // Try with TLS - seems to work
    if endpoint.starts_with("https://") {
        exporter_builder =
            exporter_builder.with_tls_config(ClientTlsConfig::new().with_enabled_roots());
    }

    let exporter = exporter_builder.build()?;

    let tracer = SdkTracerProvider::builder()
        // Customize sampling strategy
        .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
            1.0,
        ))))
        // If export trace to AWS X-Ray, you can use XrayIdGenerator
        .with_id_generator(RandomIdGenerator::default())
        .with_resource(resource())
        .with_batch_exporter(exporter)
        .build();

    // We set this in meter provider but didn't in here
    // :facepalm:
    global::set_tracer_provider(tracer.clone());

    Ok(tracer)
}

fn init_meter_provider() -> SdkMeterProvider {
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_temporality(opentelemetry_sdk::metrics::Temporality::default())
        .build()
        .unwrap();

    let reader = PeriodicReader::builder(exporter)
        .with_interval(std::time::Duration::from_secs(30))
        .build();

    // For debugging in development
    let stdout_reader =
        PeriodicReader::builder(opentelemetry_stdout::MetricExporter::default()).build();

    let meter_provider = MeterProviderBuilder::default()
        .with_resource(resource())
        .with_reader(reader)
        .with_reader(stdout_reader)
        .build();

    global::set_meter_provider(meter_provider.clone());

    meter_provider
}

fn init_tracing_subscriber(metadata: MetadataMap) -> OtelGuard {
    let tracer_provider = init_tracer_provider(metadata).expect("Initializing tracer failed");
    let meter_provider = init_meter_provider();

    let tracer = tracer_provider.tracer("tracing-otel-subscriber");

    tracing_subscriber::registry()
        // The global level filter prevents the exporter network stack
        // from reentering the globally installed OpenTelemetryLayer with
        // its own spans while exporting, as the libraries should not use
        // tracing levels below DEBUG. If the OpenTelemetry layer needs to
        // trace spans and events with higher verbosity levels, consider using
        // per-layer filtering to target the telemetry layer specifically,
        // e.g. by target matching.
        .with(tracing_subscriber::filter::LevelFilter::from_level(
            Level::DEBUG,
        ))
        .with(tracing_subscriber::fmt::layer())
        .with(MetricsLayer::new(meter_provider.clone()))
        .with(OpenTelemetryLayer::new(tracer))
        .init();

    OtelGuard {
        tracer_provider,
        meter_provider,
    }
}

struct OtelGuard {
    tracer_provider: SdkTracerProvider,
    meter_provider: SdkMeterProvider,
}

impl Drop for OtelGuard {
    fn drop(&mut self) {
        if let Err(err) = self.tracer_provider.shutdown() {
            eprintln!("{err:?}");
        }
        if let Err(err) = self.meter_provider.shutdown() {
            eprintln!("{err:?}");
        }
    }
}

#[tokio::main]
#[instrument]
async fn main() -> anyhow::Result<()> {
    // nym_bin_common::logging::setup_tracing_logger();

    dotenv().ok();
    let key = std::env::var("SIGNOZ_INGESTION_KEY").expect("SIGNOZ_INGESTION_KEY not set");
    println!("SIGNOZ_INGESTION_KEY = {}", key);

    let mut metadata = MetadataMap::new();
    metadata.insert("signoz-ingestion-key", key.parse()?);

    let _guard = init_tracing_subscriber(metadata);
    let tracer = global::tracer("sdk-example-surb-reply");
    let span = tracer.start("test_span");
    let cx = Context::current_with_span(span);
    let _guard = cx.clone().attach();

    let trace_id = cx.span().span_context().trace_id();
    warn!("Main TRACE_ID: {:?}", trace_id);

    let span = info_span!(
        "surb_reply_example_session",
        trace_id = %trace_id.to_string()
    );
    let _enter = span.enter();

    let otel_context = opentelemetry::Context::current();
    warn!("OTEL CONTEXT: {:?}", otel_context);
    let span = otel_context.span();
    let context = span.span_context();
    let trace_id = context.trace_id();
    warn!("TRACE_ID: {:?}", trace_id);
    // panic!();

    // Specify some config options
    // let config_dir: PathBuf = TempDir::new().unwrap().path().to_path_buf();
    // let storage_paths = StoragePaths::new_from_dir(&config_dir).unwrap();

    // Create the client with a storage backend, and enable it by giving it some paths. If keys
    // exists at these paths, they will be loaded, otherwise they will be generated.
    // let client = MixnetClientBuilder::new_with_default_storage(storage_paths)
    //     .await
    //     .unwrap()
    //     .build()
    //     .unwrap();

    let client_builder = MixnetClientBuilder::new_ephemeral();
    let mixnet_client = client_builder
        .request_gateway("BAF2aYpzcK9KbSS3Y7EdLisxiogkTr88FXkdL8EDNigH".to_string())
        .with_ignore_epoch_roles(true)
        .with_extended_topology(true)
        .build()?;

    // Now we connect to the mixnet, using keys now stored in the paths provided.
    let mut client = mixnet_client.connect_to_mixnet().await.unwrap();

    // Be able to get our client address
    let our_address = client.nym_address();
    println!("\nOur client nym address is: {our_address}");

    // Send a message through the mixnet to ourselves using our nym address
    client
        .send_plain_message(*our_address, "hello there")
        .await
        .unwrap();

    // we're going to parse the sender_tag (AnonymousSenderTag) from the incoming message and use it to 'reply' to ourselves instead of our Nym address.
    // we know there will be a sender_tag since the sdk sends SURBs along with messages by default.
    println!("Waiting for message\n");

    // get the actual message - discard the empty vec sent along with a potential SURB topup request
    let mut message: Vec<ReconstructedMessage> = Vec::new();
    while let Some(new_message) = client.wait_for_messages().await {
        if new_message.is_empty() {
            continue;
        }
        message = new_message;
        break;
    }

    let mut parsed = String::new();
    if let Some(r) = message.first() {
        parsed = String::from_utf8(r.message.clone()).unwrap();
    }
    // parse sender_tag: we will use this to reply to sender without needing their Nym address
    let return_recipient: AnonymousSenderTag = message[0].sender_tag.unwrap();
    println!(
        "\nReceived the following message: {parsed} \nfrom sender with surb bucket {return_recipient}"
    );

    // reply to self with it: note we use `send_str_reply` instead of `send_str`
    println!("Replying with using SURBs");
    client
        .send_reply(return_recipient, "hi an0n!")
        .await
        .unwrap();

    println!("Waiting for message (once you see it, ctrl-c to exit)\n");
    client
        .on_messages(|msg| println!("\nReceived: {}", String::from_utf8_lossy(&msg.message)))
        .await;

    Ok(())
}

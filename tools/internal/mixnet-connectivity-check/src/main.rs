// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use clap::{Args, Parser, Subcommand};
use futures::stream::StreamExt;
use nym_bin_common::output_format::OutputFormat;
use nym_bin_common::{bin_info, bin_info_owned};
use nym_crypto::asymmetric::ed25519;
use nym_network_defaults::setup_env;
use nym_sdk::mixnet::MixnetMessageSender;
use nym_sdk::{mixnet, DebugConfig};
use std::sync::OnceLock;
use std::time::Duration;
use tokio::time::timeout;

// signoz
use dotenv::dotenv;
use opentelemetry::{global, trace::TracerProvider as _, KeyValue};
use opentelemetry_otlp::tonic_types::metadata::MetadataMap;
use opentelemetry_otlp::tonic_types::transport::ClientTlsConfig;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_otlp::WithTonicConfig;
use opentelemetry_sdk::{
    metrics::{MeterProviderBuilder, PeriodicReader, SdkMeterProvider},
    trace::{RandomIdGenerator, Sampler, SdkTracerProvider},
    Resource,
};
use opentelemetry_semantic_conventions::{
    attribute::{DEPLOYMENT_ENVIRONMENT_NAME, SERVICE_VERSION},
    SCHEMA_URL,
};
use tracing_core::Level;
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

#[derive(Parser)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub(crate) struct Cli {
    /// Path pointing to an env file that configures the client.
    #[clap(short, long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    #[clap(subcommand)]
    command: Commands,
}

impl Cli {
    async fn execute(self) -> anyhow::Result<()> {
        match self.command {
            Commands::CheckConnectivity(args) => connectivity_test(args).await?,
            Commands::BuildInfo(args) => build_info(args),
        }
        Ok(())
    }
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
pub(crate) enum Commands {
    /// Attempt to run a simple connectivity test
    CheckConnectivity(ConnectivityArgs),

    /// Show build information of this binary
    BuildInfo(BuildInfoArgs),
}

#[derive(Args, Clone, Debug)]
struct ConnectivityArgs {
    #[clap(long)]
    gateway: Option<ed25519::PublicKey>,

    #[clap(long)]
    ignore_performance: bool,
}

#[derive(clap::Args, Debug)]
pub(crate) struct BuildInfoArgs {
    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

fn build_info(args: BuildInfoArgs) {
    println!("{}", args.output.format(&bin_info_owned!()))
}

async fn connectivity_test(args: ConnectivityArgs) -> anyhow::Result<()> {
    let env = mixnet::NymNetworkDetails::new_from_env();
    let mut debug_config = DebugConfig::default();
    debug_config.cover_traffic.disable_loop_cover_traffic_stream = true;
    debug_config
        .traffic
        .disable_main_poisson_packet_distribution = true;

    if args.ignore_performance {
        debug_config.topology.minimum_mixnode_performance = 0;
        debug_config.topology.minimum_gateway_performance = 0;
    };

    let client_builder = mixnet::MixnetClientBuilder::new_ephemeral()
        .network_details(env)
        .debug_config(debug_config);

    let mixnet_client = if let Some(gateway) = args.gateway {
        client_builder
            .request_gateway(gateway.to_string())
            .with_ignore_epoch_roles(true)
            .with_extended_topology(true)
            .build()?
    } else {
        client_builder.build()?
    };

    print!("connecting to mixnet... ");
    let mut client = match mixnet_client.connect_to_mixnet().await {
        Ok(client) => {
            println!("✅");
            client
        }
        Err(err) => {
            println!("❌");
            println!("failed to connect: {err}");
            return Err(err.into());
        }
    };
    let our_address = client.nym_address();

    println!("attempting to send a message to ourselves ({our_address})");

    client
        .send_plain_message(*our_address, "hello there")
        .await?;

    print!("awaiting response... ");

    match timeout(Duration::from_secs(5), client.next()).await {
        Err(_timeout) => {
            println!("❌");
            println!("timed out while waiting for the response...");
        }
        Ok(Some(received)) => match String::from_utf8(received.message) {
            Ok(message) => {
                println!("✅");
                println!("received '{message}' back!");
            }
            Err(err) => {
                println!("❌");
                println!("the received message got malformed on the way to us: {err}");
            }
        },
        Ok(None) => {
            println!("❌");
            println!("failed to receive any message back...");
        }
    }

    println!("disconnecting the client before shutting down...");
    client.disconnect().await;
    Ok(())
}

fn resource() -> Resource {
    Resource::builder()
        .with_service_name("mixnet-connectivity-check")
        .with_schema_url(
            [
                KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
                KeyValue::new(DEPLOYMENT_ENVIRONMENT_NAME, "develop"),
            ],
            SCHEMA_URL,
        )
        .build()
}

// Construct MeterProvider for MetricsLayer
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

fn init_tracer_provider(metadata: MetadataMap) -> anyhow::Result<SdkTracerProvider> {
    let endpoint = std::env::var("SIGNOZ_ENDPOINT").expect("SIGNOZ_ENDPOINT not set");

    // Configure OTLP exporter with metadata
    let mut exporter_builder = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(&endpoint)
        .with_metadata(metadata);

    // Try with TLS - seems to work
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
            Level::INFO,
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
#[tracing::instrument]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let key = std::env::var("SIGNOZ_INGESTION_KEY").expect("SIGNOZ_INGESTION_KEY not set");

    let mut metadata = MetadataMap::new();
    metadata.insert("signoz-ingestion-key", key.parse()?);

    let _guard = init_tracing_subscriber(metadata);

    let args = Cli::parse();
    setup_env(args.config_env_file.as_ref());
    let _ = args.execute().await;

    tokio::time::sleep(std::time::Duration::from_secs(4)).await;

    Ok(())
}

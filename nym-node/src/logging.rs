// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_bin_common::logging::{default_tracing_env_filter, default_tracing_fmt_layer};
use tracing_subscriber::filter::Directive;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

/// Configuration for OpenTelemetry OTLP export.
/// Fields are only read when the `otel` feature is enabled.
#[allow(dead_code)]
pub(crate) struct OtelConfig {
    /// OTLP/gRPC collector endpoint, e.g. `http://localhost:4317`
    /// or `https://ingest.eu.signoz.cloud:443` for SigNoz Cloud.
    pub endpoint: String,
    /// Service name reported to the collector (appears in SigNoz "Services" view).
    pub service_name: String,
    /// Optional SigNoz Cloud ingestion key for authenticated export.
    /// Sent as the `signoz-ingestion-key` gRPC metadata header.
    pub ingestion_key: Option<String>,
    /// Deployment environment label, e.g. `mainnet`, `sandbox`, `canary`.
    /// Attached as the `deployment.environment` OTel resource attribute.
    pub environment: String,
    /// Trace sampling ratio in 0.0..=1.0 (e.g. 0.1 = 10% of traces). Used to limit cost.
    pub sample_ratio: f64,
    /// Timeout in seconds for each OTLP export batch. Prevents unbounded blocking.
    pub export_timeout_secs: u64,
}

/// Handle returned when OTel is active.  Flushes pending spans on drop
/// to prevent telemetry loss during panics or early exits.
#[cfg(feature = "otel")]
pub(crate) struct OtelGuard {
    pub provider: opentelemetry_sdk::trace::SdkTracerProvider,
}

#[cfg(feature = "otel")]
impl Drop for OtelGuard {
    fn drop(&mut self) {
        if let Err(e) = self.provider.shutdown() {
            eprintln!("OpenTelemetry shutdown error in Drop: {e}");
        }
    }
}

pub(crate) fn granual_filtered_env() -> anyhow::Result<EnvFilter> {
    fn directive_checked(directive: impl Into<String>) -> anyhow::Result<Directive> {
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

/// Initialise the tracing subscriber stack.
///
/// When the `otel` feature is enabled **and** an `OtelConfig` is supplied, an
/// OTLP exporter layer is added and the returned `OtelGuard` must be used to
/// flush pending spans on shutdown.
#[cfg(feature = "otel")]
pub(crate) fn setup_tracing_logger(otel: Option<OtelConfig>) -> anyhow::Result<Option<OtelGuard>> {
    let stderr_layer =
        default_tracing_fmt_layer(std::io::stderr).with_filter(granual_filtered_env()?);

    cfg_if::cfg_if! {if #[cfg(feature = "tokio-console")] {
        let console_layer = console_subscriber::spawn();

        if let Some(otel_config) = otel {
            let (otel_layer, provider) = nym_bin_common::logging::init_otel_layer(
                &otel_config.service_name,
                &otel_config.endpoint,
                otel_config.ingestion_key.as_deref(),
                &otel_config.environment,
                otel_config.sample_ratio,
                otel_config.export_timeout_secs,
            ).map_err(|e| anyhow::anyhow!(
                "failed to initialise OpenTelemetry exporter (endpoint: {}, service: {}): {e}",
                otel_config.endpoint,
                otel_config.service_name,
            ))?;

            tracing_subscriber::registry()
                .with(console_layer)
                .with(stderr_layer)
                .with(otel_layer)
                .init();

            Ok(Some(OtelGuard { provider }))
        } else {
            tracing_subscriber::registry()
                .with(console_layer)
                .with(stderr_layer)
                .init();

            Ok(None)
        }
    } else {
        if let Some(otel_config) = otel {
            let (otel_layer, provider) = nym_bin_common::logging::init_otel_layer(
                &otel_config.service_name,
                &otel_config.endpoint,
                otel_config.ingestion_key.as_deref(),
                &otel_config.environment,
                otel_config.sample_ratio,
                otel_config.export_timeout_secs,
            ).map_err(|e| anyhow::anyhow!(
                "failed to initialise OpenTelemetry exporter (endpoint: {}, service: {}): {e}",
                otel_config.endpoint,
                otel_config.service_name,
            ))?;

            tracing_subscriber::registry()
                .with(stderr_layer)
                .with(otel_layer)
                .init();

            Ok(Some(OtelGuard { provider }))
        } else {
            tracing_subscriber::registry()
                .with(stderr_layer)
                .init();

            Ok(None)
        }
    }}
}

/// Non-OTel variant -- identical subscriber stack without the OTLP layer.
#[cfg(not(feature = "otel"))]
pub(crate) fn setup_tracing_logger(otel: Option<OtelConfig>) -> anyhow::Result<()> {
    let _ = otel;
    let stderr_layer =
        default_tracing_fmt_layer(std::io::stderr).with_filter(granual_filtered_env()?);

    cfg_if::cfg_if! {if #[cfg(feature = "tokio-console")] {
        let console_layer = console_subscriber::spawn();

        tracing_subscriber::registry()
            .with(console_layer)
            .with(stderr_layer)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(stderr_layer)
            .init();
    }}

    Ok(())
}

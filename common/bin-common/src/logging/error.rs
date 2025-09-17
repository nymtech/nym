// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0


#[derive(thiserror::Error, Debug)]
pub enum TracingError {
    #[error("tracing logger already initialised")]
    TracingLoggerAlreadyInitialised,

    #[cfg(feature = "otel")]
    #[error("OpenTelemetry exporter build error: {0}")]
    TracingExporterBuildError(#[from] opentelemetry_otlp::ExporterBuildError),

    #[error("Logging error: {0}")]
    TracingTryInitError(tracing_subscriber::util::TryInitError),

    #[error("{0}")]
    TracingFilterParseError(#[from] tracing_subscriber::filter::ParseError),
}

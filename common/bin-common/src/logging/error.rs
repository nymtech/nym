// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
#[cfg(feature = "otel")]
use opentelemetry_otlp::ExporterBuildError;

#[derive(thiserror::Error, Debug)]
pub enum TracingError {
    #[error("tracing logger already initialised")]
    TracingLoggerAlreadyInitialised,

    #[error("Logging error: {0}")]
    TracingTryInitError(tracing_subscriber::util::TryInitError),

    #[cfg(feature = "otel")]
    #[error("{0}")]
    TracingExporterBuildError(#[from] ExporterBuildError),

    #[error("{0}")]
    TracingFilterParseError(#[from] tracing_subscriber::filter::ParseError),
}

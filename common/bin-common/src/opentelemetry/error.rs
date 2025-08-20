// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use opentelemetry_otlp::ExporterBuildError;

#[derive(thiserror::Error, Debug)]
pub enum TracingError {
    #[error("tracing logger already initialised")]
    TracingLoggerAlreadyInitialised,

    // #[error("I/O error: {0}")]
    // IoError(#[from] std::io::Error),
    #[error("Logging error: {0}")]
    TracingTryInitError(tracing_subscriber::util::TryInitError),

    #[error("{0}")]
    TracingExporterBuildError(#[from] ExporterBuildError),

    #[error("{0}")]
    TracingFilterParseError(#[from] tracing_subscriber::filter::ParseError),
}

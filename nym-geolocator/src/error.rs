// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GeoLocatorError {
    #[error("ipinfo rate limit exceeded")]
    IpInfoRateLimit,

    #[error("ipinfo request failed: {source}")]
    IpInfoRequestFailure { source: reqwest::Error },

    #[error("ipinfo rejected the request with status {status}: {body}")]
    IpInfoRequestRejected {
        status: http::StatusCode,
        body: String,
    },

    #[error("ipinfo returned no location data for the queried address")]
    IpInfoNoLocationData,

    #[error("failed to deserialize ipinfo response: {source}")]
    IpInfoResponseDeserialisationFailure { source: serde_json::Error },
}

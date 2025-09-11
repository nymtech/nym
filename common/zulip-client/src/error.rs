// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ZulipClientError {
    #[error("failed to send request to {url}: {source}")]
    RequestSendingFailure { url: String, source: reqwest::Error },

    #[error("failed to decode received response: {source}")]
    RequestDecodeFailure { source: reqwest::Error },

    #[error("failed to build internal client: {source}")]
    ClientBuildFailure { source: reqwest::Error },

    #[error("provided url ({raw}) is malformed: {source}")]
    MalformedServerUrl {
        raw: String,
        source: url::ParseError,
    },
}

// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::vpn_api_client::types::ErrorResponse;
use nym_http_api_client::HttpClientError;

#[cfg(test)]
pub(crate) mod client;

pub mod types;

pub type NymVpnApiClientError = HttpClientError<ErrorResponse>;

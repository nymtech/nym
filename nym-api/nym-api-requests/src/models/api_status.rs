// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::time::Duration;
use utoipa::ToSchema;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct ApiHealthResponse {
    pub status: ApiStatus,
    #[serde(default)]
    pub chain_status: ChainStatus,
    pub uptime: u64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ApiStatus {
    Up,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default, schemars::JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChainStatus {
    Synced,
    #[default]
    Unknown,
    Stalled {
        #[serde(
            serialize_with = "humantime_serde::serialize",
            deserialize_with = "humantime_serde::deserialize"
        )]
        approximate_amount: Duration,
    },
}

impl ChainStatus {
    pub fn is_synced(&self) -> bool {
        matches!(self, ChainStatus::Synced)
    }
}

impl ApiHealthResponse {
    pub fn new_healthy(uptime: Duration) -> Self {
        ApiHealthResponse {
            status: ApiStatus::Up,
            chain_status: ChainStatus::Synced,
            uptime: uptime.as_secs(),
        }
    }
}

impl ApiStatus {
    pub fn is_up(&self) -> bool {
        matches!(self, ApiStatus::Up)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct SignerInformationResponse {
    pub cosmos_address: String,

    pub identity: String,

    pub announce_address: String,

    pub verification_key: Option<String>,
}

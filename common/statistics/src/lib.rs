// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use error::StatsError;

pub mod error;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StatsMessage {
    pub stats_data: Vec<StatsServiceData>,
    pub interval_seconds: u32,
    pub timestamp: String,
}

impl StatsMessage {
    pub fn to_json(&self) -> Result<String, StatsError> {
        Ok(serde_json::to_string(self)?)
    }

    pub fn from_json(s: &str) -> Result<Self, StatsError> {
        Ok(serde_json::from_str(s)?)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StatsServiceData {
    pub requested_service: String,
    pub request_bytes: u32,
    pub response_bytes: u32,
}

impl StatsServiceData {
    pub fn new(requested_service: String, request_bytes: u32, response_bytes: u32) -> Self {
        StatsServiceData {
            requested_service,
            request_bytes,
            response_bytes,
        }
    }
}

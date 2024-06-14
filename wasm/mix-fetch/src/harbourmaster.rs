// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// this exists inside mix-fetch rather than being made into repo-wide library since it's a temporary solution
// until the API is finalised and endpoints are moved to nym-api

use async_trait::async_trait;
use nym_http_api_client::{ApiClient, HttpClientError, NO_PARAMS};
use serde::Deserialize;

pub use nym_http_api_client::Client;

pub type HarbourMasterApiError = HttpClientError;

mod routes {
    pub const API_VERSION: &str = "v1";

    pub const SERVICES: &str = "services";

    pub const NEW: &str = "new";
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait HarbourMasterApiClientExt: ApiClient {
    // since it's a temporary thing don't worry about paging.
    async fn get_services_new(&self) -> Result<PagedResult<ServiceNew>, HarbourMasterApiError> {
        self.get_json(
            &[routes::API_VERSION, routes::SERVICES, routes::NEW],
            NO_PARAMS,
        )
        .await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl HarbourMasterApiClientExt for Client {}

// https://gitlab.nymte.ch/nym/shipyard-test-and-earn/-/blob/main/harbour-master/src/http/mod.rs#L13
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct PagedResult<T> {
    pub page: u32,
    pub size: u32,
    pub total: i32,
    pub items: Vec<T>,
}

// https://gitlab.nymte.ch/nym/shipyard-test-and-earn/-/blob/main/harbour-master/src/http/services.rs#L32
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct ServiceNew {
    pub service_provider_client_id: String,
    pub ip_address: String,
    pub last_successful_ping_utc: String,
    pub last_updated_utc: String,
    pub routing_score: f32,
}

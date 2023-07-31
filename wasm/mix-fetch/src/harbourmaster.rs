// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// this exists inside mix-fetch rather than being made into repo-wide library since it's a temporary solution
// until the API is finalised and endpoints are moved to nym-api

use reqwest::{IntoUrl, Response, StatusCode};
use serde::Deserialize;
use thiserror::Error;
use url::Url;

mod routes {
    pub const API_VERSION: &str = "v1";

    pub const SERVICES: &str = "services";

    pub const NEW: &str = "new";
}

// most of it is copied from the nym-api client
type PathSegments<'a> = &'a [&'a str];
type Params<'a, K, V> = &'a [(K, V)];

const NO_PARAMS: Params<'_, &'_ str, &'_ str> = &[];

#[derive(Debug, Error)]
pub enum HarbourMasterApiError {
    #[error("there was an issue with the REST request: {source}")]
    ReqwestClientError {
        #[from]
        source: reqwest::Error,
    },

    #[error("not found")]
    NotFound,

    #[error("request failed with error message: {0}")]
    GenericRequestFailure(String),
}

pub struct Client {
    url: Url,
    reqwest_client: reqwest::Client,
}

impl Client {
    pub fn new<U: IntoUrl>(url: U) -> Result<Self, HarbourMasterApiError> {
        let reqwest_client = reqwest::Client::new();
        Ok(Self {
            url: url.into_url()?,
            reqwest_client,
        })
    }

    async fn send_get_request<K, V>(
        &self,
        path: PathSegments<'_>,
        params: Params<'_, K, V>,
    ) -> Result<Response, HarbourMasterApiError>
    where
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let url = create_api_url(&self.url, path, params);
        Ok(self.reqwest_client.get(url).send().await?)
    }

    async fn query_harbourmaster<T, K, V>(
        &self,
        path: PathSegments<'_>,
        params: Params<'_, K, V>,
    ) -> Result<T, HarbourMasterApiError>
    where
        for<'a> T: Deserialize<'a>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let res = self.send_get_request(path, params).await?;
        if res.status().is_success() {
            Ok(res.json().await?)
        } else if res.status() == StatusCode::NOT_FOUND {
            Err(HarbourMasterApiError::NotFound)
        } else {
            Err(HarbourMasterApiError::GenericRequestFailure(
                res.text().await?,
            ))
        }
    }

    // since it's a temporary thing don't worry about paging.
    pub async fn get_services_new(&self) -> Result<PagedResult<ServiceNew>, HarbourMasterApiError> {
        self.query_harbourmaster(
            &[routes::API_VERSION, routes::SERVICES, routes::NEW],
            NO_PARAMS,
        )
        .await
    }
}

fn create_api_url<K: AsRef<str>, V: AsRef<str>>(
    base: &Url,
    segments: PathSegments<'_>,
    params: Params<'_, K, V>,
) -> Url {
    let mut url = base.clone();
    let mut path_segments = url
        .path_segments_mut()
        .expect("provided validator url does not have a base!");
    for segment in segments {
        let segment = segment.strip_prefix('/').unwrap_or(segment);
        let segment = segment.strip_suffix('/').unwrap_or(segment);

        path_segments.push(segment);
    }
    drop(path_segments);

    if !params.is_empty() {
        url.query_pairs_mut().extend_pairs(params);
    }

    url
}

// https://gitlab.nymte.ch/nym/shipyard-test-and-earn/-/blob/main/harbour-master/src/http/mod.rs#L13
#[derive(Debug, Deserialize)]
pub struct PagedResult<T> {
    pub page: u32,
    pub size: u32,
    pub total: i32,
    pub items: Vec<T>,
}

// https://gitlab.nymte.ch/nym/shipyard-test-and-earn/-/blob/main/harbour-master/src/http/services.rs#L32
#[derive(Debug, Deserialize)]
pub struct ServiceNew {
    pub service_provider_client_id: String,
    pub ip_address: String,
    pub last_successful_ping_utc: String,
    pub last_updated_utc: String,
    pub routing_score: f32,
}

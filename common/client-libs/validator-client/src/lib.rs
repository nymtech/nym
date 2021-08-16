// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

pub use crate::error::ValidatorClientError;
use crate::models::{QueryRequest, QueryResponse};
use log::error;
use mixnet_contract::{
    GatewayBond, IdentityKey, LayerDistribution, MixNodeBond, PagedGatewayResponse,
    PagedMixnodeResponse,
};
use rand::{seq::SliceRandom, thread_rng};
use serde::Deserialize;
use url::Url;

mod error;
mod models;
#[cfg(feature = "nymd-client")]
pub mod nymd;
pub(crate) mod serde_helpers;
pub mod validator_api;

// Implement caching with a global hashmap that has two fields, queryresponse and as_at, there is a side process
pub struct Config {
    initial_rest_servers: Vec<Url>,
    mixnet_contract_address: String,
    mixnode_page_limit: Option<u32>,
    gateway_page_limit: Option<u32>,
}

impl Config {
    pub fn new<S: Into<String>>(
        rest_servers_available_base_urls: Vec<String>,
        mixnet_contract_address: S,
    ) -> Self {
        let initial_rest_servers = rest_servers_available_base_urls
            .iter()
            .map(|base_url| Url::parse(base_url).expect("Bad validator URL"))
            .collect();
        Config {
            initial_rest_servers,
            mixnet_contract_address: mixnet_contract_address.into(),
            mixnode_page_limit: None,
            gateway_page_limit: None,
        }
    }

    pub fn with_mixnode_page_limit(mut self, limit: Option<u32>) -> Config {
        self.mixnode_page_limit = limit;
        self
    }

    pub fn with_gateway_page_limit(mut self, limit: Option<u32>) -> Config {
        self.gateway_page_limit = limit;
        self
    }
}

pub struct Client {
    config: Config,
    // Currently it seems the client is independent of the url hence a single instance seems to be fine
    reqwest_client: reqwest::Client,
    validator_api_client: validator_api::Client,
}

impl Client {
    pub fn new(config: Config) -> Self {
        let reqwest_client = reqwest::Client::new();
        let validator_api_client = validator_api::Client::new();

        // client is only ever created on process startup, so a panic here is fine as it implies
        // invalid config. And that can only happen if an user was messing with it by themselves.
        if config.initial_rest_servers.is_empty() {
            panic!("no validator servers provided")
        }

        Client {
            config,
            reqwest_client,
            validator_api_client,
        }
    }

    pub fn available_validators_rest_urls(&self) -> Vec<Url> {
        self.config.initial_rest_servers.clone()
    }

    fn base_query_path(&self, url: &str) -> String {
        format!(
            "{}/wasm/contract/{}/smart",
            url, self.config.mixnet_contract_address
        )
    }

    // async fn latest_block(&self) -> Block {
    //     let path = format!("{}/block", self.available_validators_rest_urls[0]);
    //     let response = self.reqwest_client.get(path).send().await?.json().await?;
    // }

    async fn query_validators<T>(
        &self,
        query: String,
        use_validator_api: bool,
    ) -> Result<T, ValidatorClientError>
    where
        for<'a> T: Deserialize<'a>,
    {
        let mut failed = 0;
        let sleep_secs = 5;
        // Randomly select a validator to query, keep querying and shuffling until we get a response
        let mut validator_urls = self.available_validators_rest_urls().clone();

        // This will never exit
        // JS: Shouldn't it have some sort of maximum attempts counter to return an error at some point?
        loop {
            validator_urls.as_mut_slice().shuffle(&mut thread_rng());
            for url in validator_urls.iter() {
                let res = if use_validator_api {
                    Ok(self
                        .validator_api_client
                        .query_validator_api(query.clone(), url)
                        .await?)
                } else {
                    self.query_validator(query.clone(), url).await
                };
                match res {
                    Ok(res) => return Ok(res),
                    Err(e) => {
                        failed += 1;
                        error!("{}", e);
                        error!("Total failed requests {}", failed);
                    }
                }
            }
            error!(
                "No validators available out of {} attempted! Will try again in {} seconds. Listing all attempted:",
                validator_urls.len(), sleep_secs
            );
            for url in validator_urls.iter() {
                error!("{}", url)
            }
            // Went with only wasm_timer so we can avoid features on the lib, and pulling in tokio
            fluvio_wasm_timer::Delay::new(Duration::from_secs(sleep_secs)).await?;
        }
    }

    async fn query_validator<T>(
        &self,
        query: String,
        validator_url: &Url,
    ) -> Result<T, ValidatorClientError>
    where
        for<'a> T: Deserialize<'a>,
    {
        let query_url = format!(
            "{}/{}?encoding=base64",
            self.base_query_path(validator_url.as_str()),
            query
        );

        let query_response: QueryResponse<T> = self
            .reqwest_client
            .get(query_url)
            .send()
            .await?
            .json()
            .await?;

        match query_response {
            QueryResponse::Ok(smart_res) => Ok(smart_res.result.smart),
            QueryResponse::Error(err) => Err(ValidatorClientError::ValidatorError(err.error)),
            QueryResponse::CodedError(err) => {
                Err(ValidatorClientError::ValidatorError(format!("{}", err)))
            }
        }
    }

    async fn get_mix_nodes_paged(
        &self,
        start_after: Option<IdentityKey>,
    ) -> Result<PagedMixnodeResponse, ValidatorClientError> {
        let query_content_json = serde_json::to_string(&QueryRequest::GetMixNodes {
            limit: self.config.mixnode_page_limit,
            start_after,
        })
        .expect("serde was incorrectly implemented on QueryRequest::GetMixNodes!");

        // we need to encode our json request
        let query_content = base64::encode(query_content_json);

        self.query_validators(query_content, false).await
    }

    pub async fn get_mix_nodes(&self) -> Result<Vec<MixNodeBond>, ValidatorClientError> {
        let mut mixnodes = Vec::new();
        let mut start_after = None;
        loop {
            let mut paged_response = self.get_mix_nodes_paged(start_after.take()).await?;
            mixnodes.append(&mut paged_response.nodes);

            if let Some(start_after_res) = paged_response.start_next_after {
                start_after = Some(start_after_res)
            } else {
                break;
            }
        }

        Ok(mixnodes)
    }

    pub async fn get_cached_mix_nodes(&self) -> Result<Vec<MixNodeBond>, ValidatorClientError> {
        let query_content = format!(
            "{}{}",
            validator_api::VALIDATOR_API_CACHE_VERSION.to_string(),
            validator_api::VALIDATOR_API_MIXNODES.to_string()
        );
        self.query_validators(query_content, true).await
    }

    async fn get_gateways_paged(
        &self,
        start_after: Option<IdentityKey>,
    ) -> Result<PagedGatewayResponse, ValidatorClientError> {
        let query_content_json = serde_json::to_string(&QueryRequest::GetGateways {
            limit: self.config.gateway_page_limit,
            start_after,
        })
        .expect("serde was incorrectly implemented on QueryRequest::GetGateways!");

        // we need to encode our json request
        let query_content = base64::encode(query_content_json);

        self.query_validators(query_content, false).await
    }

    pub async fn get_gateways(&self) -> Result<Vec<GatewayBond>, ValidatorClientError> {
        let mut gateways = Vec::new();
        let mut start_after = None;
        loop {
            let mut paged_response = self.get_gateways_paged(start_after.take()).await?;
            gateways.append(&mut paged_response.nodes);

            if let Some(start_after_res) = paged_response.start_next_after {
                start_after = Some(start_after_res)
            } else {
                break;
            }
        }

        Ok(gateways)
    }

    pub async fn get_cached_gateways(&self) -> Result<Vec<GatewayBond>, ValidatorClientError> {
        let query_content = format!(
            "{}{}",
            validator_api::VALIDATOR_API_CACHE_VERSION.to_string(),
            validator_api::VALIDATOR_API_GATEWAYS.to_string()
        );
        self.query_validators(query_content, true).await
    }

    pub async fn get_layer_distribution(&self) -> Result<LayerDistribution, ValidatorClientError> {
        // serialization of an empty enum can't fail...
        let query_content_json =
            serde_json::to_string(&QueryRequest::LayerDistribution {}).unwrap();

        // we need to encode our json request
        let query_content = base64::encode(query_content_json);

        self.query_validators(query_content, false).await
    }
}

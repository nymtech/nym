// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::{QueryRequest, QueryResponse};
use crate::ValidatorClientError::ValidatorError;
use core::fmt::{self, Display, Formatter};
use mixnet_contract::{GatewayBond, HumanAddr, MixNodeBond, PagedGatewayResponse, PagedResponse};
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::Deserialize;
use std::collections::VecDeque;

mod models;
pub(crate) mod serde_helpers;

#[derive(Debug)]
pub enum ValidatorClientError {
    ReqwestClientError(reqwest::Error),
    ValidatorError(String),
}

impl std::error::Error for ValidatorClientError {}

impl From<reqwest::Error> for ValidatorClientError {
    fn from(err: reqwest::Error) -> Self {
        ValidatorClientError::ReqwestClientError(err)
    }
}

impl Display for ValidatorClientError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ValidatorClientError::ReqwestClientError(err) => {
                write!(f, "there was an issue with the REST request - {}", err)
            }
            ValidatorClientError::ValidatorError(err) => {
                write!(f, "there was an issue with the validator client - {}", err)
            }
        }
    }
}

fn permute_validators(validators: VecDeque<String>) -> VecDeque<String> {
    // even in the best case scenario in the mainnet world, we're not going to have more than ~100 validators,
    // hence conversions from and to Vec are fine
    let mut vec = Vec::from(validators);

    vec.shuffle(&mut thread_rng());

    vec.into()
}

pub struct Config {
    initial_rest_servers: Vec<String>,
    mixnet_contract_address: String,
    mixnode_page_limit: Option<u32>,
    gateway_page_limit: Option<u32>,
}

impl Config {
    pub fn new<S: Into<String>>(
        rest_servers_available_base_urls: Vec<String>,
        mixnet_contract_address: S,
    ) -> Self {
        Config {
            initial_rest_servers: rest_servers_available_base_urls,
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

    available_validators_rest_urls: VecDeque<String>,
    failed_queries: usize,
}

impl Client {
    pub fn new(config: Config) -> Self {
        let reqwest_client = reqwest::Client::new();

        // client is only ever created on process startup, so a panic here is fine as it implies
        // invalid config. And that can only happen if an user was messing with it by themselves.
        if config.initial_rest_servers.is_empty() {
            panic!("no validator servers provided")
        }

        let mut available_validators_rest_urls = config.initial_rest_servers.clone().into();
        available_validators_rest_urls = permute_validators(available_validators_rest_urls);

        Client {
            config,
            reqwest_client,
            available_validators_rest_urls,
            failed_queries: 0,
        }
    }

    fn permute_validators(&mut self) {
        if self.available_validators_rest_urls.len() == 1 {
            return;
        }
        self.available_validators_rest_urls =
            permute_validators(std::mem::take(&mut self.available_validators_rest_urls));
    }

    fn base_query_path(&self) -> String {
        format!(
            "{}/wasm/contract/{}/smart",
            self.available_validators_rest_urls[0], self.config.mixnet_contract_address
        )
    }

    async fn query_validators<T>(&mut self, query: String) -> Result<T, ValidatorClientError>
    where
        for<'a> T: Deserialize<'a>,
    {
        // if we fail to query the first validator, push it to the back
        let res = self.query_front_validator(query).await;

        // don't bother doing any fancy validator switches if we only have 1 validator to choose from
        if self.available_validators_rest_urls.len() > 1 {
            if res.is_err() {
                let front = self.available_validators_rest_urls.pop_front().unwrap();
                self.available_validators_rest_urls.push_back(front);
                self.failed_queries += 1;
            }

            // if we exhausted all of available validators, permute the set, maybe the old ones
            // are working again next time we try
            if self.failed_queries == self.available_validators_rest_urls.len() {
                self.permute_validators();
                self.failed_queries = 0
            }
        }

        res
    }

    async fn query_front_validator<T>(&self, query: String) -> Result<T, ValidatorClientError>
    where
        for<'a> T: Deserialize<'a>,
    {
        let query_url = format!("{}/{}?encoding=base64", self.base_query_path(), query);

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
            QueryResponse::CodedError(err) => Err(ValidatorError(format!("{}", err))),
        }
    }

    async fn get_mix_nodes_paged(
        &mut self,
        start_after: Option<HumanAddr>,
    ) -> Result<PagedResponse, ValidatorClientError> {
        let query_content_json = serde_json::to_string(&QueryRequest::GetMixNodes {
            limit: self.config.mixnode_page_limit,
            start_after,
        })
        .expect("serde was incorrectly implemented on QueryRequest::GetMixNodes!");

        // we need to encode our json request
        let query_content = base64::encode(query_content_json);

        self.query_validators(query_content).await
    }

    pub async fn get_mix_nodes(&mut self) -> Result<Vec<MixNodeBond>, ValidatorClientError> {
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

    async fn get_gateways_paged(
        &mut self,
        start_after: Option<HumanAddr>,
    ) -> Result<PagedGatewayResponse, ValidatorClientError> {
        let query_content_json = serde_json::to_string(&QueryRequest::GetGateways {
            limit: self.config.gateway_page_limit,
            start_after,
        })
        .expect("serde was incorrectly implemented on QueryRequest::GetGateways!");

        // we need to encode our json request
        let query_content = base64::encode(query_content_json);

        self.query_validators(query_content).await
    }

    pub async fn get_gateways(&mut self) -> Result<Vec<GatewayBond>, ValidatorClientError> {
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
}

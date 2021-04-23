// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::{QueryRequest, QueryResponse};
use crate::ValidatorClientError::ValidatorError;
use core::fmt::{self, Display, Formatter};
use mixnet_contract::{GatewayBond, HumanAddr, MixNodeBond, PagedGatewayResponse, PagedResponse};
use serde::Deserialize;

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

pub struct Config {
    rpc_server_base_url: String,
    mixnet_contract_address: String,
    mixnode_page_limit: Option<u32>,
    gateway_page_limit: Option<u32>,
}

impl Config {
    pub fn new<S: Into<String>>(rpc_server_base_url: S, mixnet_contract_address: S) -> Self {
        Config {
            rpc_server_base_url: rpc_server_base_url.into(),
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
    reqwest_client: reqwest::Client,
}

impl Client {
    pub fn new(config: Config) -> Self {
        let reqwest_client = reqwest::Client::new();
        Client {
            config,
            reqwest_client,
        }
    }

    fn base_query_path(&self) -> String {
        format!(
            "{}/wasm/contract/{}/smart",
            self.config.rpc_server_base_url, self.config.mixnet_contract_address
        )
    }

    async fn query_validator<T>(&self, query: String) -> Result<T, ValidatorClientError>
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
        &self,
        start_after: Option<HumanAddr>,
    ) -> Result<PagedResponse, ValidatorClientError> {
        let query_content_json = serde_json::to_string(&QueryRequest::GetMixNodes {
            limit: self.config.mixnode_page_limit,
            start_after,
        })
        .expect("serde was incorrectly implemented on QueryRequest::GetMixNodes!");

        // we need to encode our json request
        let query_content = base64::encode(query_content_json);

        self.query_validator(query_content).await
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

    async fn get_gateways_paged(
        &self,
        start_after: Option<HumanAddr>,
    ) -> Result<PagedGatewayResponse, ValidatorClientError> {
        let query_content_json = serde_json::to_string(&QueryRequest::GetGateways {
            limit: self.config.gateway_page_limit,
            start_after,
        })
        .expect("serde was incorrectly implemented on QueryRequest::GetGateways!");

        // we need to encode our json request
        let query_content = base64::encode(query_content_json);

        self.query_validator(query_content).await
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
}

// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::models::gateway::GatewayRegistrationInfo;
use crate::models::mixmining::{BatchMixStatus, MixStatus};
use crate::models::mixnode::MixRegistrationInfo;
use crate::models::topology::Topology;
use crate::rest_requests::{
    ActiveTopologyGet, ActiveTopologyGetResponse, BatchMixStatusPost, GatewayRegisterPost,
    MixRegisterPost, MixStatusPost, NodeUnregisterDelete, ReputationPatch, RestRequest,
    RestRequestError, TopologyGet, TopologyGetResponse,
};
use serde::Deserialize;
use std::fmt::{self, Display, Formatter};

pub mod models;
pub mod rest_requests;

// for ease of use
type Result<T> = std::result::Result<T, ValidatorClientError>;

const MAX_SANE_UNEXPECTED_PRINT: usize = 100;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", untagged)]
pub(crate) enum ErrorResponses {
    Error(ErrorResponse),
    Unexpected(String),
}

impl From<ErrorResponses> for ValidatorClientError {
    fn from(err: ErrorResponses) -> Self {
        match err {
            ErrorResponses::Error(err_message) => {
                ValidatorClientError::ValidatorError(err_message.error)
            }
            ErrorResponses::Unexpected(received) => {
                ValidatorClientError::UnexpectedResponse(received)
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ErrorResponse {
    error: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OkResponse {
    ok: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", untagged)]
pub(crate) enum DefaultRestResponse {
    Ok(OkResponse),
    Error(ErrorResponses),
}

#[derive(Debug)]
pub enum ValidatorClientError {
    RestRequestError(RestRequestError),
    ReqwestClientError(reqwest::Error),
    ValidatorError(String),
    UnexpectedResponse(String),
}

impl From<RestRequestError> for ValidatorClientError {
    fn from(err: RestRequestError) -> Self {
        ValidatorClientError::RestRequestError(err)
    }
}

impl From<reqwest::Error> for ValidatorClientError {
    fn from(err: reqwest::Error) -> Self {
        ValidatorClientError::ReqwestClientError(err)
    }
}

impl Display for ValidatorClientError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ValidatorClientError::RestRequestError(err) => {
                write!(f, "could not prepare the REST request - {}", err)
            }
            ValidatorClientError::ReqwestClientError(err) => {
                write!(f, "there was an issue with the REST request - {}", err)
            }
            ValidatorClientError::ValidatorError(err) => {
                write!(f, "there was an issue with the validator client - {}", err)
            }
            ValidatorClientError::UnexpectedResponse(received) => {
                if received.len() < MAX_SANE_UNEXPECTED_PRINT {
                    write!(
                        f,
                        "received data was completely unexpected. got: {}",
                        received
                    )
                } else {
                    write!(
                        f,
                        "received data was completely unexpected. got: {}...",
                        received
                            .chars()
                            .take(MAX_SANE_UNEXPECTED_PRINT)
                            .collect::<String>()
                    )
                }
            }
        }
    }
}

pub struct Config {
    base_url: String,
}

impl Config {
    pub fn new<S: Into<String>>(base_url: S) -> Self {
        Config {
            base_url: base_url.into(),
        }
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

    async fn make_rest_request<R: RestRequest>(
        &self,
        request: R,
    ) -> Result<R::ExpectedJsonResponse> {
        let mut req_builder = self
            .reqwest_client
            .request(R::METHOD, request.url().clone());

        if let Some(json_payload) = request.json_payload() {
            // if applicable, attach payload
            req_builder = req_builder.json(json_payload)
        }
        Ok(req_builder.send().await?.json().await?)
    }

    pub async fn register_mix(&self, mix_registration_info: MixRegistrationInfo) -> Result<()> {
        let req = MixRegisterPost::new(
            &self.config.base_url,
            None,
            None,
            Some(mix_registration_info),
        )?;
        match self.make_rest_request(req).await? {
            DefaultRestResponse::Ok(_) => Ok(()),
            DefaultRestResponse::Error(err) => Err(err.into()),
        }
    }

    pub async fn register_gateway(
        &self,
        gateway_registration_info: GatewayRegistrationInfo,
    ) -> Result<()> {
        let req = GatewayRegisterPost::new(
            &self.config.base_url,
            None,
            None,
            Some(gateway_registration_info),
        )?;
        match self.make_rest_request(req).await? {
            DefaultRestResponse::Ok(ok_res) => {
                if ok_res.ok {
                    Ok(())
                } else {
                    Err(ValidatorClientError::ValidatorError(
                        "received ok response with false".into(),
                    ))
                }
            }
            DefaultRestResponse::Error(err) => Err(err.into()),
        }
    }

    pub async fn unregister_node(&self, node_id: &str) -> Result<()> {
        let req =
            NodeUnregisterDelete::new(&self.config.base_url, Some(vec![node_id]), None, None)?;

        match self.make_rest_request(req).await? {
            DefaultRestResponse::Ok(ok_res) => {
                if ok_res.ok {
                    Ok(())
                } else {
                    Err(ValidatorClientError::ValidatorError(
                        "received ok response with false".into(),
                    ))
                }
            }
            DefaultRestResponse::Error(err) => Err(err.into()),
        }
    }

    pub async fn set_reputation(&self, node_id: &str, new_reputation: i64) -> Result<()> {
        let new_rep_string = new_reputation.to_string();
        let query_param_values = vec![&*new_rep_string];
        let query_param_keys = ReputationPatch::query_param_keys();

        let query_params = query_param_keys
            .into_iter()
            .zip(query_param_values.into_iter())
            .collect();

        let req = ReputationPatch::new(
            &self.config.base_url,
            Some(vec![node_id]),
            Some(query_params),
            None,
        )?;
        match self.make_rest_request(req).await? {
            DefaultRestResponse::Ok(ok_res) => {
                if ok_res.ok {
                    Ok(())
                } else {
                    Err(ValidatorClientError::ValidatorError(
                        "received ok response with false".into(),
                    ))
                }
            }
            DefaultRestResponse::Error(err) => Err(err.into()),
        }
    }

    pub async fn get_topology(&self) -> Result<Topology> {
        let req = TopologyGet::new(&self.config.base_url, None, None, None)?;
        match self.make_rest_request(req).await? {
            TopologyGetResponse::Ok(topology) => Ok(topology),
            TopologyGetResponse::Error(err) => Err(err.into()),
        }
    }

    pub async fn get_active_topology(&self) -> Result<Topology> {
        let req = ActiveTopologyGet::new(&self.config.base_url, None, None, None)?;
        match self.make_rest_request(req).await? {
            ActiveTopologyGetResponse::Ok(topology) => Ok(topology),
            ActiveTopologyGetResponse::Error(err) => Err(err.into()),
        }
    }

    pub async fn post_mixmining_status(&self, status: MixStatus) -> Result<()> {
        let req = MixStatusPost::new(&self.config.base_url, None, None, Some(status))?;
        match self.make_rest_request(req).await? {
            DefaultRestResponse::Ok(ok_res) => {
                if ok_res.ok {
                    Ok(())
                } else {
                    Err(ValidatorClientError::ValidatorError(
                        "received ok response with false".into(),
                    ))
                }
            }
            DefaultRestResponse::Error(err) => Err(err.into()),
        }
    }

    pub async fn post_batch_mixmining_status(&self, batch_status: BatchMixStatus) -> Result<()> {
        let req = BatchMixStatusPost::new(&self.config.base_url, None, None, Some(batch_status))?;
        match self.make_rest_request(req).await? {
            DefaultRestResponse::Ok(ok_res) => {
                if ok_res.ok {
                    Ok(())
                } else {
                    Err(ValidatorClientError::ValidatorError(
                        "received ok response with false".into(),
                    ))
                }
            }
            DefaultRestResponse::Error(err) => Err(err.into()),
        }
    }
}

#[cfg(test)]
pub(crate) fn client_test_fixture(base_url: &str) -> Client {
    Client {
        config: Config::new(base_url),
        reqwest_client: reqwest::Client::new(),
    }
}

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

use reqwest::{Method, Url};
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::{self, Display, Formatter};

pub(crate) use active_topology_get::{
    Request as ActiveTopologyGet, Response as ActiveTopologyGetResponse,
};
pub(crate) use gateway_register_post::Request as GatewayRegisterPost;
pub(crate) use mix_mining_batch_status_post::Request as BatchMixStatusPost;
pub(crate) use mix_mining_status_post::Request as MixStatusPost;
pub(crate) use mix_register_post::Request as MixRegisterPost;
pub(crate) use node_unregister_delete::Request as NodeUnregisterDelete;
pub(crate) use set_reputation_patch::Request as ReputationPatch;
pub(crate) use topology_get::{Request as TopologyGet, Response as TopologyGetResponse};

pub mod active_topology_get;
pub mod gateway_register_post;
pub mod mix_mining_batch_status_post;
pub mod mix_mining_status_post;
pub mod mix_register_post;
pub mod node_unregister_delete;
pub mod set_reputation_patch;
pub mod topology_get;

type PathParam<'a> = &'a str;
type QueryParam<'a> = (&'a str, &'a str);

#[derive(Debug)]
pub enum RestRequestError {
    InvalidPathParams,
    InvalidQueryParams,
    NoPayloadProvided,
    MalformedUrl(String),
}

impl Display for RestRequestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RestRequestError::InvalidPathParams => write!(
                f,
                "invalid number of path parameters was provided or they were malformed"
            ),
            RestRequestError::InvalidQueryParams => write!(
                f,
                "invalid number of query parameters was provided or they were malformed"
            ),
            RestRequestError::NoPayloadProvided => {
                write!(f, "no request payload was provided while it was expected")
            }
            RestRequestError::MalformedUrl(url) => {
                write!(f, "tried to make request to malformed url ({})", url)
            }
        }
    }
}

pub(crate) trait RestRequest {
    // 'GET', 'POST', 'DELETE', etc.
    const METHOD: Method;
    const RELATIVE_PATH: &'static str;

    type JsonPayload: Serialize + Sized;
    type ExpectedJsonResponse: DeserializeOwned + Sized;

    fn new(
        base_url: &str,
        path_params: Option<Vec<PathParam>>,
        query_params: Option<Vec<QueryParam>>,
        body_payload: Option<Self::JsonPayload>,
    ) -> Result<Self, RestRequestError>
    where
        Self: Sized;

    fn url(&self) -> &Url;

    fn json_payload(&self) -> Option<&Self::JsonPayload> {
        None
    }

    fn query_param_keys() -> Vec<&'static str> {
        Vec::new()
    }
}

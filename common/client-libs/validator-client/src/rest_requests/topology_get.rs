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

use crate::models::topology::Topology;
use crate::rest_requests::{PathParam, QueryParam, RestRequest, RestRequestError};
use crate::ErrorResponses;
use reqwest::{Method, Url};
use serde::Deserialize;

pub struct Request {
    url: Url,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", untagged)]
pub(crate) enum Response {
    Ok(Topology),
    Error(ErrorResponses),
}

impl RestRequest for Request {
    const METHOD: Method = Method::GET;
    const RELATIVE_PATH: &'static str = "/api/mixmining/topology";

    type JsonPayload = ();
    type ExpectedJsonResponse = Response;

    fn new(
        base_url: &str,
        _: Option<Vec<PathParam>>,
        _: Option<Vec<QueryParam>>,
        _: Option<Self::JsonPayload>,
    ) -> Result<Self, RestRequestError> {
        let url = Url::parse(&format!("{}{}", base_url, Self::RELATIVE_PATH))
            .map_err(|err| RestRequestError::MalformedUrl(err.to_string()))?;

        Ok(Request { url })
    }

    fn url(&self) -> &Url {
        &self.url
    }
}

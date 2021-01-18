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
use crate::rest_requests::{PathParam, QueryParam, RESTRequest, RESTRequestError};
use crate::DefaultRESTResponse;
use reqwest::{Method, Url};

pub struct Request {
    url: Url,
    payload: GatewayRegistrationInfo,
}

impl RESTRequest for Request {
    const METHOD: Method = Method::POST;
    const RELATIVE_PATH: &'static str = "/api/mixmining/register/gateway";

    type JsonPayload = GatewayRegistrationInfo;
    type ExpectedJsonResponse = DefaultRESTResponse;

    fn new(
        base_url: &str,
        _: Option<Vec<PathParam>>,
        _: Option<Vec<QueryParam>>,
        body_payload: Option<Self::JsonPayload>,
    ) -> Result<Self, RESTRequestError> {
        let payload = body_payload.ok_or(RESTRequestError::NoPayloadProvided)?;
        let url = Url::parse(&format!("{}{}", base_url, Self::RELATIVE_PATH))
            .map_err(|err| RESTRequestError::MalformedUrl(err.to_string()))?;

        Ok(Request { url, payload })
    }

    fn url(&self) -> &Url {
        &self.url
    }

    fn json_payload(&self) -> Option<&Self::JsonPayload> {
        Some(&self.payload)
    }
}

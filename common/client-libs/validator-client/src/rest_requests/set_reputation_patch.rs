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

use crate::rest_requests::{PathParam, QueryParam, RestRequest, RestRequestError};
use crate::DefaultRestResponse;
use reqwest::{Method, Url};

pub struct Request {
    url: Url,
}

impl RestRequest for Request {
    const METHOD: Method = Method::PATCH;
    const RELATIVE_PATH: &'static str = "/api/mixmining/reputation";
    type JsonPayload = ();
    type ExpectedJsonResponse = DefaultRestResponse;

    fn new(
        base_url: &str,
        path_params: Option<Vec<PathParam>>,
        query_params: Option<Vec<QueryParam>>,
        _: Option<Self::JsonPayload>,
    ) -> Result<Self, RestRequestError> {
        // set reputation requires single path param - the node id
        // and single query param - what reputation should it be set to
        let path_params = path_params.ok_or(RestRequestError::InvalidPathParams)?;
        if path_params.len() != 1 {
            return Err(RestRequestError::InvalidPathParams);
        }

        let query_params = query_params.ok_or(RestRequestError::InvalidQueryParams)?;
        if query_params.len() != 1 {
            return Err(RestRequestError::InvalidQueryParams);
        }

        // <base_url>/api/mixmining/reputation/{id}
        let base = format!("{}{}/{}", base_url, Self::RELATIVE_PATH, path_params[0]);

        let url = Url::parse_with_params(&base, query_params)
            .map_err(|err| RestRequestError::MalformedUrl(err.to_string()))?;

        Ok(Request { url })
    }

    fn url(&self) -> &Url {
        &self.url
    }

    fn query_param_keys() -> Vec<&'static str> {
        vec!["reputation"]
    }
}

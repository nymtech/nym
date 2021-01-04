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

use crate::rest_requests::{PathParam, QueryParam, RESTRequest, RESTRequestError};
use crate::DefaultRESTResponse;
use reqwest::{Method, Url};

pub struct Request {
    url: Url,
}

impl RESTRequest for Request {
    const METHOD: Method = Method::DELETE;
    const RELATIVE_PATH: &'static str = "/api/mixmining/register";
    type JsonPayload = ();
    type ExpectedJsonResponse = DefaultRESTResponse;

    fn new(
        base_url: &str,
        path_params: Option<Vec<PathParam>>,
        _: Option<Vec<QueryParam>>,
        _: Option<Self::JsonPayload>,
    ) -> Result<Self, RESTRequestError> {
        // node unregister requires single path param - the node id
        let path_params = path_params.ok_or(RESTRequestError::InvalidPathParams)?;
        if path_params.len() != 1 {
            return Err(RESTRequestError::InvalidPathParams);
        }
        // <base_url>/api/mixmining/register/{id}
        let base = format!("{}{}/{}", base_url, Self::RELATIVE_PATH, path_params[0]);

        let url =
            Url::parse(&base).map_err(|err| RESTRequestError::MalformedUrl(err.to_string()))?;

        Ok(Request { url })
    }

    fn url(&self) -> &Url {
        &self.url
    }
}

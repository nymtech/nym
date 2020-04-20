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

use reqwest::Response;

pub struct Request {
    base_url: String,
    path: String,
}

pub trait HealthCheckRequester {
    fn new(base_url: String) -> Self;
    fn get(&self) -> Result<Response, reqwest::Error>;
}

impl HealthCheckRequester for Request {
    fn new(base_url: String) -> Self {
        Request {
            base_url,
            path: "/api/healthcheck".to_string(),
        }
    }

    fn get(&self) -> Result<Response, reqwest::Error> {
        let url = format!("{}{}", self.base_url, self.path);
        reqwest::get(&url)
    }
}

#[cfg(test)]
mod healthcheck_requests {
    use super::*;

    #[cfg(test)]
    use mockito::mock;

    #[cfg(test)]
    mod on_a_400_status {
        use super::*;

        #[test]
        #[should_panic]
        fn it_returns_an_error() {
            let _m = mock("GET", "/api/healthcheck").with_status(400).create();
            let req = Request::new(mockito::server_url());
            assert!(req.get().is_err());
            _m.assert();
        }
    }

    #[cfg(test)]
    mod on_a_200 {
        use super::*;

        #[test]
        fn it_returns_a_response_with_200_status() {
            let _m = mock("GET", "/api/healthcheck").with_status(200).create();
            let req = Request::new(mockito::server_url());
            assert!(req.get().is_ok());
            _m.assert();
        }
    }
}

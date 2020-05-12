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

use super::{DirectoryGetRequest, DirectoryRequest};
use serde::{Deserialize, Serialize};

const PATH: &str = "/api/healthcheck";

#[derive(Deserialize, Serialize)]
pub struct HealthCheckResponse {
    pub ok: bool,
}

pub struct Request {
    base_url: String,
    path: String,
}

impl DirectoryRequest for Request {
    fn url(&self) -> String {
        format!("{}{}", self.base_url, self.path)
    }
}

impl DirectoryGetRequest for Request {
    type JSONResponse = HealthCheckResponse;

    fn new(base_url: &str) -> Self {
        Request {
            base_url: base_url.to_string(),
            path: PATH.to_string(),
        }
    }
}

#[cfg(test)]
mod healthcheck_requests {
    use crate::client_test_fixture;
    use mockito::mock;

    #[cfg(test)]
    mod on_a_400_status {
        use super::*;

        #[tokio::test]
        async fn it_returns_an_error() {
            let _m = mock("GET", "/api/healthcheck").with_status(400).create();
            let client = client_test_fixture(&mockito::server_url());
            let res = client.get_healthcheck().await;
            assert!(res.is_err());
            _m.assert();
        }
    }

    #[cfg(test)]
    mod on_a_200 {
        use super::*;

        #[tokio::test]
        async fn it_returns_a_response_with_200_status() {
            let json = r#"{
                "ok": true
            }"#;
            let _m = mock("GET", "/api/healthcheck")
                .with_status(200)
                .with_body(json)
                .create();
            let client = client_test_fixture(&mockito::server_url());
            let res = client.get_healthcheck().await;
            assert!(res.unwrap().ok);
            _m.assert();
        }
    }
}

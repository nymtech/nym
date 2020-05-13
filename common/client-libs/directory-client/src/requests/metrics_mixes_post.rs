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

use super::{DirectoryPostRequest, DirectoryRequest};
use crate::metrics::MixMetric;

const PATH: &str = "/api/metrics/mixes";

pub struct Request {
    base_url: String,
    path: String,
    payload: MixMetric,
}

impl DirectoryRequest for Request {
    fn url(&self) -> String {
        format!("{}{}", self.base_url, self.path)
    }
}

impl DirectoryPostRequest for Request {
    type Payload = MixMetric;
    fn json_payload(&self) -> &MixMetric {
        &self.payload
    }

    fn new(base_url: &str, payload: Self::Payload) -> Self {
        Request {
            base_url: base_url.to_string(),
            path: PATH.to_string(),
            payload,
        }
    }
}

#[cfg(test)]
mod metrics_post_request {
    use super::*;
    use crate::client_test_fixture;
    use mockito::mock;

    #[cfg(test)]
    mod on_a_400_status {
        use super::*;

        #[tokio::test]
        async fn it_returns_an_error() {
            let _m = mock("POST", PATH).with_status(400).create();
            let client = client_test_fixture(&mockito::server_url());
            let result = client.post_mix_metrics(fixtures::new_metric()).await;
            assert_eq!(400, result.unwrap().status());
            _m.assert();
        }
    }

    #[cfg(test)]
    mod on_a_200 {
        use super::*;
        #[tokio::test]
        async fn it_returns_a_response_with_200() {
            let json = fixtures::mix_metrics_response_json();
            let _m = mock("POST", "/api/metrics/mixes")
                .with_status(201)
                .with_body(json)
                .create();
            let client = client_test_fixture(&mockito::server_url());
            let result = client.post_mix_metrics(fixtures::new_metric()).await;
            assert!(result.is_ok());
            _m.assert();
        }
    }

    #[cfg(test)]
    mod fixtures {
        use crate::metrics::MixMetric;

        pub fn new_metric() -> MixMetric {
            MixMetric {
                pub_key: "abc".to_string(),
                received: 666,
                sent: Default::default(),
            }
        }

        #[cfg(test)]
        pub fn mix_metrics_response_json() -> String {
            r#"
              {
                "pubKey": "OwOqwWjh_IlnaWS2PxO6odnhNahOYpRCkju50beQCTA=",
                "sent": {
                  "35.178.213.77:1789": 1,
                  "52.56.99.196:1789": 2
                },
                "received": 10,
                "timestamp": 1576061080635800000
              }
            "#
            .to_string()
        }
    }
}

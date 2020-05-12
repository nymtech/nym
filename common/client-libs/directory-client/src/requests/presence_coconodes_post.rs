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
use crate::presence::coconodes::CocoPresence;

const PATH: &str = "/api/presence/coconodes";

pub struct Request {
    base_url: String,
    path: String,
    payload: CocoPresence,
}

impl DirectoryRequest for Request {
    fn url(&self) -> String {
        format!("{}{}", self.base_url, self.path)
    }
}

impl DirectoryPostRequest for Request {
    type Payload = CocoPresence;
    fn json_payload(&self) -> &CocoPresence {
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
mod presence_coconodes_post_request {
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
            let presence = fixtures::new_presence();
            let result = client.post_coconode_presence(presence).await;
            assert_eq!(400, result.unwrap().status());
            _m.assert();
        }
    }

    #[cfg(test)]
    mod on_a_200 {
        use super::*;
        #[tokio::test]
        async fn it_returns_a_response_with_201() {
            let json = r#"{
                          "ok": true
                      }"#;
            let _m = mock("POST", PATH).with_status(201).with_body(json).create();
            let client = client_test_fixture(&mockito::server_url());
            let presence = fixtures::new_presence();
            let result = client.post_coconode_presence(presence).await;
            assert!(result.is_ok());
            _m.assert();
        }
    }

    #[cfg(test)]
    mod fixtures {
        use crate::presence::coconodes::CocoPresence;

        pub fn new_presence() -> CocoPresence {
            CocoPresence {
                location: "foomp".to_string(),
                host: "foo.com".to_string(),
                pub_key: "abc".to_string(),
                last_seen: 666,
                version: "0.2.0".to_string(),
            }
        }
    }
}

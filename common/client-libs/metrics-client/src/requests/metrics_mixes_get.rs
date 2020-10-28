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

const PATH: &str = "/api/metrics/mixes";

pub struct Request {
    base_url: String,
    path: String,
}

impl Request {
    pub(crate) fn new(base_url: &str) -> Self {
        Request {
            base_url: base_url.to_string(),
            path: PATH.to_string(),
        }
    }

    pub(crate) fn url(&self) -> String {
        format!("{}{}", self.base_url, self.path)
    }
}

#[cfg(test)]
mod metrics_get_request {
    use super::*;
    use crate::client_test_fixture;
    use mockito::mock;

    #[cfg(test)]
    mod on_a_400_status {
        use super::*;

        #[tokio::test]
        async fn it_returns_an_error() {
            let _m = mock("GET", PATH).with_status(400).create();
            let client = client_test_fixture(&mockito::server_url());
            let result = client.get_mix_metrics().await;
            assert!(result.is_err());
            _m.assert();
        }
    }

    #[cfg(test)]
    mod on_a_200 {
        use super::*;
        #[tokio::test]
        async fn it_returns_a_response_with_200_status_and_a_correct_metrics() {
            let json = fixtures::mix_metrics_response_json();
            let _m = mock("GET", PATH).with_status(200).with_body(json).create();
            let client = client_test_fixture(&mockito::server_url());
            let result = client.get_mix_metrics().await;
            let unwrapped = result.unwrap();

            assert_eq!(10, unwrapped.first().unwrap().received);
            _m.assert();
        }
    }

    #[cfg(test)]
    mod fixtures {
        #[cfg(test)]
        pub fn mix_metrics_response_json() -> &'static str {
            r#"[
              {
                "pubKey": "OwOqwWjh_IlnaWS2PxO6odnhNahOYpRCkju50beQCTA=",
                "sent": {
                  "35.178.213.77:1789": 1,
                  "52.56.99.196:1789": 2
                },
                "received": 10,
                "timestamp": 1576061080635800000
              },
              {
                "pubKey": "zSob16499jT7C3S3ky4GihNOjlU6aLfSRkf1xAxOwV0=",
                "sent": {
                  "3.8.176.11:1789": 2,
                  "35.178.212.193:1789": 2
                },
                "received": 4,
                "timestamp": 1576061080806225700
              },
              {
                "pubKey": "nkkrUjgL8UJk05QydvWvFSvtRB6nmeV8RMvH5540J3s=",
                "sent": {
                  "3.9.12.238:1789": 3
                },
                "received": 3,
                "timestamp": 1576061080894667300
              },
              {
                "pubKey": "whHuBuEc6zyOZOquKbuATaH4Crml61V_3Y-MztpWhF4=",
                "sent": {
                  "3.9.12.238:1789": 5,
                  "35.176.155.107:1789": 3
                },
                "received": 8,
                "timestamp": 1576061081254846500
              },
              {
                "pubKey": "vCdpFc0NvW0NSqsuTxtjFtiSY35aXesgT3JNA8sSIXk=",
                "sent": {
                  "35.178.213.77:1789": 4,
                  "52.56.99.196:1789": 6
                },
                "received": 19,
                "timestamp": 1576061081371549000
              },
              {
                "pubKey": "vk5Sr-Xyi0cTbugACv8U42ZJ6hs6cGDox0rpmXY94Fc=",
                "sent": {
                  "3.8.176.11:1789": 4,
                  "35.178.212.193:1789": 2
                },
                "received": 6,
                "timestamp": 1576061081498404900
              },
              {
                "pubKey": "OwOqwWjh_IlnaWS2PxO6odnhNahOYpRCkju50beQCTA=",
                "sent": {
                  "35.178.213.77:1789": 2,
                  "52.56.99.196:1789": 3
                },
                "received": 6,
                "timestamp": 1576061081637625000
              },
              {
                "pubKey": "zSob16499jT7C3S3ky4GihNOjlU6aLfSRkf1xAxOwV0=",
                "sent": {
                  "3.8.176.11:1789": 5,
                  "35.178.212.193:1789": 4
                },
                "received": 9,
                "timestamp": 1576061081805484800
              },
              {
                "pubKey": "nkkrUjgL8UJk05QydvWvFSvtRB6nmeV8RMvH5540J3s=",
                "sent": {
                  "3.9.12.238:1789": 4,
                  "35.176.155.107:1789": 4
                },
                "received": 8,
                "timestamp": 1576061081896562400
              },
              {
                "pubKey": "whHuBuEc6zyOZOquKbuATaH4Crml61V_3Y-MztpWhF4=",
                "sent": {
                  "3.9.12.238:1789": 2,
                  "35.176.155.107:1789": 4
                },
                "received": 6,
                "timestamp": 1576061079255938600
              },
              {
                "pubKey": "vCdpFc0NvW0NSqsuTxtjFtiSY35aXesgT3JNA8sSIXk=",
                "sent": {
                  "35.178.213.77:1789": 6
                },
                "received": 10,
                "timestamp": 1576061079370829300
              },
              {
                "pubKey": "vk5Sr-Xyi0cTbugACv8U42ZJ6hs6cGDox0rpmXY94Fc=",
                "sent": {
                  "3.8.176.11:1789": 2,
                  "35.178.212.193:1789": 5
                },
                "received": 7,
                "timestamp": 1576061079497993200
              },
              {
                "pubKey": "OwOqwWjh_IlnaWS2PxO6odnhNahOYpRCkju50beQCTA=",
                "sent": {
                  "35.178.213.77:1789": 5,
                  "52.56.99.196:1789": 2
                },
                "received": 13,
                "timestamp": 1576061079637208600
              },
              {
                "pubKey": "zSob16499jT7C3S3ky4GihNOjlU6aLfSRkf1xAxOwV0=",
                "sent": {
                  "3.8.176.11:1789": 5,
                  "35.178.212.193:1789": 4
                },
                "received": 9,
                "timestamp": 1576061079806557200
              },
              {
                "pubKey": "nkkrUjgL8UJk05QydvWvFSvtRB6nmeV8RMvH5540J3s=",
                "sent": {
                  "3.9.12.238:1789": 2,
                  "35.176.155.107:1789": 7
                },
                "received": 9,
                "timestamp": 1576061079895988000
              },
              {
                "pubKey": "whHuBuEc6zyOZOquKbuATaH4Crml61V_3Y-MztpWhF4=",
                "sent": {
                  "3.9.12.238:1789": 3,
                  "35.176.155.107:1789": 2
                },
                "received": 5,
                "timestamp": 1576061080255701500
              },
              {
                "pubKey": "vCdpFc0NvW0NSqsuTxtjFtiSY35aXesgT3JNA8sSIXk=",
                "sent": {
                  "35.178.213.77:1789": 3,
                  "52.56.99.196:1789": 3
                },
                "received": 7,
                "timestamp": 1576061080370956300
              },
              {
                "pubKey": "vk5Sr-Xyi0cTbugACv8U42ZJ6hs6cGDox0rpmXY94Fc=",
                "sent": {
                  "3.8.176.11:1789": 5,
                  "35.178.212.193:1789": 1
                },
                "received": 6,
                "timestamp": 1576061080501732900
              }
            ]"#
        }
    }
}

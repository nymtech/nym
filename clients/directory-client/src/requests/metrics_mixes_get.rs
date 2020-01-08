use crate::metrics::PersistedMixMetric;

pub struct Request {
    base_url: String,
    path: String,
}

pub trait MetricsMixRequester {
    fn new(base_url: String) -> Self;
    fn get(&self) -> Result<Vec<PersistedMixMetric>, reqwest::Error>;
}

impl MetricsMixRequester for Request {
    fn new(base_url: String) -> Self {
        Request {
            base_url,
            path: "/api/metrics/mixes".to_string(),
        }
    }

    fn get(&self) -> Result<Vec<PersistedMixMetric>, reqwest::Error> {
        let url = format!("{}{}", self.base_url, self.path);
        let mix_metric_vec = reqwest::get(&url)?.json()?;
        Ok(mix_metric_vec)
    }
}

#[cfg(test)]
mod metrics_get_request {
    use super::*;

    #[cfg(test)]
    use mockito::mock;

    #[cfg(test)]
    mod on_a_400_status {
        use super::*;

        #[test]
        #[should_panic]
        fn it_returns_an_error() {
            let _m = mock("GET", "/api/metrics/mixes").with_status(400).create();
            let req = Request::new(mockito::server_url());
            req.get().unwrap();
            _m.assert();
        }
    }

    #[cfg(test)]
    mod on_a_200 {
        use super::*;
        #[test]
        fn it_returns_a_response_with_200_status_and_a_correct_topology() {
            let json = fixtures::mix_metrics_response_json();
            let _m = mock("GET", "/api/metrics/mixes")
                .with_status(200)
                .with_body(json)
                .create();
            let req = Request::new(mockito::server_url());
            let result = req.get();
            assert_eq!(true, result.is_ok());
            assert_eq!(
                1576061080635800000,
                result.unwrap().first().unwrap().timestamp
            );
            _m.assert();
        }
    }

    #[cfg(test)]
    mod fixtures {
        #[cfg(test)]
        pub fn mix_metrics_response_json() -> String {
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
            .to_string()
        }
    }
}

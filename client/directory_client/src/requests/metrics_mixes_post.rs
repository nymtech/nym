use crate::metrics::MixMetric;
use reqwest::Response;

pub struct Request {
    base_url: String,
    path: String,
}

pub trait MetricsMixPoster {
    fn new(base_url: String) -> Self;
    fn post(&self, metric: &MixMetric) -> Result<Response, reqwest::Error>;
}

impl MetricsMixPoster for Request {
    fn new(base_url: String) -> Self {
        Request {
            base_url,
            path: "/api/metrics/mixes".to_string(),
        }
    }

    fn post(&self, metric: &MixMetric) -> Result<Response, reqwest::Error> {
        let url = format!("{}{}", self.base_url, self.path);
        let client = reqwest::Client::new();
        let mix_metric_vec = client.post(&url).json(&metric).send()?;
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
        fn it_returns_an_error() {
            let _m = mock("POST", "/api/metrics/mixes").with_status(400).create();
            let req = Request::new(mockito::server_url());
            let metric = fixtures::new_metric();
            let result = req.post(&metric);
            assert_eq!(400, result.unwrap().status());
            _m.assert();
        }
    }

    #[cfg(test)]
    mod on_a_200 {
        use super::*;
        #[test]
        fn it_returns_a_response_with_200() {
            let json = fixtures::mix_metrics_response_json();
            let _m = mock("POST", "/api/metrics/mixes")
                .with_status(201)
                .with_body(json)
                .create();
            let req = Request::new(mockito::server_url());
            let metric = fixtures::new_metric();
            let result = req.post(&metric);
            assert_eq!(true, result.is_ok());
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

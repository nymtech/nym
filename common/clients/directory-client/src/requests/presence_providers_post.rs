use crate::presence::providers::MixProviderPresence;
use reqwest::Response;

pub struct Request {
    base_url: String,
    path: String,
}

pub trait PresenceMixProviderPoster {
    fn new(base_url: String) -> Self;
    fn post(&self, presence: &MixProviderPresence) -> Result<Response, reqwest::Error>;
}

impl PresenceMixProviderPoster for Request {
    fn new(base_url: String) -> Self {
        Request {
            base_url,
            path: "/api/presence/mixproviders".to_string(),
        }
    }

    fn post(&self, presence: &MixProviderPresence) -> Result<Response, reqwest::Error> {
        let url = format!("{}{}", self.base_url, self.path);
        let client = reqwest::Client::new();
        let p = client.post(&url).json(&presence).send()?;
        Ok(p)
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
            let _m = mock("POST", "/api/presence/mixproviders")
                .with_status(400)
                .create();
            let req = Request::new(mockito::server_url());
            let presence = fixtures::new_presence();
            let result = req.post(&presence);
            assert_eq!(400, result.unwrap().status());
            _m.assert();
        }
    }

    #[cfg(test)]
    mod on_a_200 {
        use super::*;
        #[test]
        fn it_returns_a_response_with_201() {
            let json = r#"{
                          "ok": true
                      }"#;
            let _m = mock("POST", "/api/presence/mixproviders")
                .with_status(201)
                .with_body(json)
                .create();
            let req = Request::new(mockito::server_url());
            let presence = fixtures::new_presence();
            let result = req.post(&presence);
            assert_eq!(true, result.is_ok());
            _m.assert();
        }
    }
    #[cfg(test)]
    mod fixtures {
        use crate::presence::providers::MixProviderPresence;

        pub fn new_presence() -> MixProviderPresence {
            MixProviderPresence {
                location: "foomp".to_string(),
                client_listener: "foo.com".to_string(),
                mixnet_listener: "foo.com".to_string(),
                pub_key: "abc".to_string(),
                registered_clients: vec![],
                last_seen: 0,
                version: "0.1.0".to_string(),
            }
        }
    }
}

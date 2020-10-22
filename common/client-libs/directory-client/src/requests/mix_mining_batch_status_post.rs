use super::{DirectoryPostRequest, DirectoryRequest};
use crate::mixmining::BatchMixStatus;

const PATH: &str = "/api/mixmining/batch";

pub struct Request {
    base_url: String,
    path: String,
    payload: BatchMixStatus,
}

impl DirectoryRequest for Request {
    fn url(&self) -> String {
        format!("{}{}", self.base_url, self.path)
    }
}

impl DirectoryPostRequest for Request {
    type Payload = BatchMixStatus;
    fn new(base_url: &str, payload: Self::Payload) -> Self {
        Request {
            base_url: base_url.to_string(),
            path: PATH.to_string(),
            payload,
        }
    }

    fn json_payload(&self) -> &BatchMixStatus {
        &self.payload
    }
}

#[cfg(test)]
mod batch_mix_status_post_request {
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
            let result = client
                .post_batch_mixmining_status(fixtures::new_status())
                .await;
            assert_eq!(400, result.unwrap().status());
            _m.assert();
        }
    }

    #[cfg(test)]
    mod on_a_201 {
        use super::*;

        #[tokio::test]
        async fn it_returns_a_response_with_201() {
            let json = r#"{
                "ok": true
            }"#;
            let _m = mock("POST", "/api/mixmining/batch")
                .with_status(201)
                .with_body(json)
                .create();
            let client = client_test_fixture(&mockito::server_url());
            let result = client
                .post_batch_mixmining_status(fixtures::new_status())
                .await;
            assert!(result.is_ok());
            _m.assert();
        }
    }

    #[cfg(test)]
    mod fixtures {
        use crate::mixmining::{BatchMixStatus, MixStatus};

        pub fn new_status() -> BatchMixStatus {
            BatchMixStatus {
                status: vec![MixStatus {
                    pub_key: "abc".to_string(),
                    ip_version: "4".to_string(),
                    up: true,
                }],
            }
        }
    }
}

use super::{DirectoryPostRequest, DirectoryRequest};
use crate::mixmining::MixStatus;

const PATH: &str = "/api/mixmining";

pub struct Request {
    base_url: String,
    path: String,
    payload: MixStatus,
}

impl DirectoryRequest for Request {
    fn url(&self) -> String {
        format!("{}{}", self.base_url, self.path)
    }
}

impl DirectoryPostRequest for Request {
    type Payload = MixStatus;
    fn json_payload(&self) -> &MixStatus {
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
mod mix_status_post_request {
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
            let result = client.post_mixmining_status(fixtures::new_status()).await;
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
            let _m = mock("POST", "/api/mixmining")
                .with_status(201)
                .with_body(json)
                .create();
            let client = client_test_fixture(&mockito::server_url());
            let result = client.post_mixmining_status(fixtures::new_status()).await;
            assert!(result.is_ok());
            _m.assert();
        }
    }

    #[cfg(test)]
    mod fixtures {
        use directory_client_models::mixmining::MixStatus;

        pub fn new_status() -> MixStatus {
            MixStatus {
                pub_key: "abc".to_string(),
                ip_version: "4".to_string(),
                up: true,
            }
        }
    }
}

use crate::models::mixmining::BatchMixStatus;
use crate::rest_requests::{PathParam, QueryParam, RestRequest, RestRequestError};
use crate::DefaultRestResponse;
use reqwest::{Method, Url};

pub struct Request {
    url: Url,
    payload: BatchMixStatus,
}

impl RestRequest for Request {
    const METHOD: Method = Method::POST;
    const RELATIVE_PATH: &'static str = "/api/mixmining/batch";
    type JsonPayload = BatchMixStatus;
    type ExpectedJsonResponse = DefaultRestResponse;

    fn new(
        base_url: &str,
        _: Option<Vec<PathParam>>,
        _: Option<Vec<QueryParam>>,
        body_payload: Option<Self::JsonPayload>,
    ) -> Result<Self, RestRequestError> {
        let payload = body_payload.ok_or(RestRequestError::NoPayloadProvided)?;
        let url = Url::parse(&format!("{}{}", base_url, Self::RELATIVE_PATH))
            .map_err(|err| RestRequestError::MalformedUrl(err.to_string()))?;
        Ok(Request { url, payload })
    }

    fn url(&self) -> &Url {
        &self.url
    }

    fn json_payload(&self) -> Option<&Self::JsonPayload> {
        Some(&self.payload)
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
            let _m = mock("POST", Request::RELATIVE_PATH)
                .with_status(400)
                .create();
            let client = client_test_fixture(&mockito::server_url());
            let result = client
                .post_batch_mixmining_status(fixtures::new_status())
                .await;
            assert!(result.is_err());
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
        use crate::models::mixmining::{BatchMixStatus, MixStatus};

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

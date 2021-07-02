pub mod error;

use crate::validator_api::error::ValidatorAPIClientError;

use serde::Deserialize;

pub struct Client {
    reqwest_client: reqwest::Client,
}

impl Client {
    pub fn new() -> Self {
        let reqwest_client = reqwest::Client::new();
        Self { reqwest_client }
    }

    pub async fn query_validator<T>(
        &self,
        query: String,
        validator_url: &str,
    ) -> Result<T, ValidatorAPIClientError>
    where
        for<'a> T: Deserialize<'a>,
    {
        let query_url = format!("{}/{}", validator_url, query);
        Ok(self
            .reqwest_client
            .get(query_url)
            .send()
            .await?
            .json()
            .await?)
    }
}

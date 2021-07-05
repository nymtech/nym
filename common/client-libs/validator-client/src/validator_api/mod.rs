pub mod error;

use crate::validator_api::error::ValidatorAPIClientError;

use serde::Deserialize;

// TODO: This should be linked to the validator-api as well
pub(crate) const MIXNODES_QUERY: &str = "v1/mixnodes";
pub(crate) const GATEWAYS_QUERY: &str = "v1/gateways";

pub struct Client {
    reqwest_client: reqwest::Client,
}

impl Client {
    pub fn new() -> Self {
        let reqwest_client = reqwest::Client::new();
        Self { reqwest_client }
    }

    pub async fn query_validator_api<T>(
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

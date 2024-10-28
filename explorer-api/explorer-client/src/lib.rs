#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

use reqwest::StatusCode;
use thiserror::Error;
use url::Url;

// Re-export request types
pub use nym_explorer_api_requests::{
    Location, PrettyDetailedGatewayBond, PrettyDetailedMixNodeBond,
};

// Paths
const API_VERSION: &str = "v1";
const TMP: &str = "tmp";
const UNSTABLE: &str = "unstable";
const MIXNODES: &str = "mix-nodes";
const GATEWAYS: &str = "gateways";

#[cfg(not(target_arch = "wasm32"))]
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Error)]
pub enum ExplorerApiError {
    #[error("REST request error: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("URL parse error: {0}")]
    UrlParseError(#[from] url::ParseError),

    #[error("not found")]
    NotFound,

    #[error("request failure: {0}")]
    RequestFailure(String),
}

pub struct ExplorerClient {
    url: Url,
    client: reqwest::Client,
}

impl ExplorerClient {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(url: url::Url) -> Result<Self, ExplorerApiError> {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()?;
        Ok(Self { client, url })
    }

    #[cfg(target_arch = "wasm32")]
    pub fn new(url: url::Url) -> Result<Self, ExplorerApiError> {
        let client = reqwest::Client::builder().build()?;
        Ok(Self { client, url })
    }

    async fn send_get_request(
        &self,
        paths: &[&str],
    ) -> Result<reqwest::Response, ExplorerApiError> {
        let url = combine_url(self.url.clone(), paths)?;
        log::trace!("Sending GET request {url:?}");
        Ok(self.client.get(url).send().await?)
    }

    async fn query_explorer_api<T>(&self, paths: &[&str]) -> Result<T, ExplorerApiError>
    where
        T: std::fmt::Debug,
        T: for<'a> serde::Deserialize<'a>,
    {
        let response = self.send_get_request(paths).await?;
        if response.status().is_success() {
            let res = response.json::<T>().await?;
            log::trace!("Got response: {res:?}");
            Ok(res)
        } else if response.status() == StatusCode::NOT_FOUND {
            Err(ExplorerApiError::NotFound)
        } else {
            Err(ExplorerApiError::RequestFailure(response.text().await?))
        }
    }

    pub async fn get_mixnodes(&self) -> Result<Vec<PrettyDetailedMixNodeBond>, ExplorerApiError> {
        self.query_explorer_api(&[API_VERSION, MIXNODES]).await
    }

    pub async fn get_gateways(&self) -> Result<Vec<PrettyDetailedGatewayBond>, ExplorerApiError> {
        self.query_explorer_api(&[API_VERSION, GATEWAYS]).await
    }

    pub async fn unstable_get_gateways(
        &self,
    ) -> Result<Vec<PrettyDetailedGatewayBond>, ExplorerApiError> {
        self.query_explorer_api(&[API_VERSION, TMP, UNSTABLE, GATEWAYS])
            .await
    }
}

fn combine_url(mut base_url: Url, paths: &[&str]) -> Result<Url, ExplorerApiError> {
    {
        let mut segments = base_url.path_segments_mut().expect("failed to parse url");
        for path in paths {
            segments.push(path);
        }
    }
    Ok(base_url)
}

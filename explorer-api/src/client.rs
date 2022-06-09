use network_defaults::{default_api_endpoints, default_nymd_endpoints, DEFAULT_NETWORK};
use reqwest::Url;
use std::sync::Arc;
use validator_client::nymd::QueryNymdClient;

// since this is just a query client, we don't need any locking mechanism to keep sequence numbers in check
// nor we need to access any of its methods taking mutable reference (like updating api URL)
// when that becomes a requirement, we would simply put an extra RwLock (or Mutex) in here

#[derive(Clone)]
pub(crate) struct ThreadsafeValidatorClient(
    pub(crate) Arc<validator_client::Client<QueryNymdClient>>,
);

impl ThreadsafeValidatorClient {
    pub(crate) fn new() -> Self {
        new_validator_client()
    }

    pub(crate) fn api_endpoint(&self) -> &Url {
        self.0.validator_api.current_url()
    }
}

pub(crate) fn new_validator_client() -> ThreadsafeValidatorClient {
    let nymd_url = default_nymd_endpoints()[0].clone();
    let api_url = default_api_endpoints()[0].clone();

    let client_config = validator_client::Config::new(DEFAULT_NETWORK, nymd_url, api_url);

    ThreadsafeValidatorClient(Arc::new(
        validator_client::Client::new_query(client_config).expect("Failed to connect to nymd!"),
    ))
}

use network_defaults::{
    var_names::{API_VALIDATOR, NYMD_VALIDATOR},
    NymNetworkDetails,
};
use reqwest::Url;
use std::{str::FromStr, sync::Arc};
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
    let nymd_url = Url::from_str(&std::env::var(NYMD_VALIDATOR).expect("nymd validator not set"))
        .expect("nymd validator not in url format");
    let api_url = Url::from_str(&std::env::var(API_VALIDATOR).expect("nymd validator not set"))
        .expect("nymd validator not in url format");

    let details = NymNetworkDetails::new_from_env();
    let client_config = validator_client::Config::try_from_nym_network_details(&details)
        .expect("failed to construct valid validator client config with the provided network")
        .with_urls(nymd_url, api_url);

    ThreadsafeValidatorClient(Arc::new(
        validator_client::Client::new_query(client_config).expect("Failed to connect to nymd!"),
    ))
}

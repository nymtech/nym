use crate::nymd::error::NymdError;
use crate::nymd::{Config as ClientConfig, NymdClient, QueryNymdClient};
use crate::ApiClient;
use network_defaults::all::Network;

use colored::Colorize;
use core::fmt;
use itertools::Itertools;
use std::collections::HashMap;
use std::hash::BuildHasher;
use std::time::Duration;
use tokio::time::timeout;
use url::Url;

const MAX_URLS_TESTED: usize = 200;
const CONNECTION_TEST_TIMEOUT_SEC: u64 = 2;

// Run connection tests for all specified nymd and api urls. These are all run concurrently.
pub async fn run_validator_connection_test<H: BuildHasher + 'static>(
    nymd_urls: impl Iterator<Item = (Network, Url)>,
    api_urls: impl Iterator<Item = (Network, Url)>,
    mixnet_contract_address: HashMap<Network, cosmrs::AccountId, H>,
) -> (
    HashMap<Network, Vec<(Url, bool)>>,
    HashMap<Network, Vec<(Url, bool)>>,
) {
    // Setup all the clients for the connection tests
    let connection_test_clients =
        setup_connection_tests(nymd_urls, api_urls, mixnet_contract_address);

    // Run all tests concurrently
    let connection_results = futures::future::join_all(
        connection_test_clients
            .into_iter()
            .take(MAX_URLS_TESTED)
            .map(ClientForConnectionTest::run_connection_check),
    )
    .await;

    // Seperate and collect results into HashMaps
    (
        extract_and_collect_results_into_map(&connection_results, &UrlType::Nymd),
        extract_and_collect_results_into_map(&connection_results, &UrlType::Api),
    )
}

fn setup_connection_tests<H: BuildHasher + 'static>(
    nymd_urls: impl Iterator<Item = (Network, Url)>,
    api_urls: impl Iterator<Item = (Network, Url)>,
    mixnet_contract_address: HashMap<Network, cosmrs::AccountId, H>,
) -> impl Iterator<Item = ClientForConnectionTest> {
    let nymd_connection_test_clients = nymd_urls.filter_map(move |(network, url)| {
        let address = mixnet_contract_address
            .get(&network)
            .expect("No configured contract address")
            .clone();
        let config = ClientConfig::try_from_nym_network_details(&network.details())
            .expect("failed to create valid nymd client config");

        if let Ok(mut client) = NymdClient::<QueryNymdClient>::connect(config, url.as_str()) {
            // possibly redundant, but lets just leave it here
            client.set_mixnet_contract_address(address);
            Some(ClientForConnectionTest::Nymd(
                network,
                url,
                Box::new(client),
            ))
        } else {
            None
        }
    });

    let api_connection_test_clients = api_urls.map(|(network, url)| {
        ClientForConnectionTest::Api(network, url.clone(), ApiClient::new(url))
    });

    nymd_connection_test_clients.chain(api_connection_test_clients)
}

fn extract_and_collect_results_into_map(
    connection_results: &[ConnectionResult],
    url_type: &UrlType,
) -> HashMap<Network, Vec<(Url, bool)>> {
    connection_results
        .iter()
        .filter(|c| &c.url_type() == url_type)
        .map(|c| {
            let (network, url, result) = c.result();
            (network.clone(), (url.clone(), *result))
        })
        .into_group_map()
}

async fn test_nymd_connection(
    network: Network,
    url: &Url,
    client: &NymdClient<QueryNymdClient>,
) -> ConnectionResult {
    let result = match timeout(
        Duration::from_secs(CONNECTION_TEST_TIMEOUT_SEC),
        client.get_mixnet_contract_version(),
    )
    .await
    {
        Ok(Err(NymdError::TendermintError(e))) => {
            // If we get a tendermint-rpc error, we classify the node as not contactable
            log::debug!(
                "Checking: nymd_url: {network}: {url}: {}: {}",
                "failed".red(),
                e
            );
            false
        }
        Ok(Err(NymdError::AbciError(code, log))) => {
            // We accept the mixnet contract not found as ok from a connection standpoint. This happens
            // for example on a pre-launch network.
            log::debug!(
                "Checking: nymd_url: {network}: {url}: {}, but with abci error: {code}: {log}",
                "success".green()
            );
            code == 18
        }
        Ok(Err(error @ NymdError::NoContractAddressAvailable)) => {
            log::debug!(
                "Checking: nymd_url: {network}: {url}: {}: {error}",
                "failed".red()
            );
            false
        }
        Ok(Err(e)) => {
            // For any other error, we're optimistic and just try anyway.
            log::debug!(
                "Checking: nymd_url: {network}: {url}: {}, but with error: {e}",
                "success".green()
            );
            true
        }
        Ok(Ok(_)) => {
            log::debug!(
                "Checking: nymd_url: {network}: {url}: {}",
                "success".green()
            );
            true
        }
        Err(e) => {
            log::debug!(
                "Checking: nymd_url: {network}: {url}: {}: {e}",
                "failed".red()
            );
            false
        }
    };
    ConnectionResult::Nymd(network, url.clone(), result)
}

async fn test_api_connection(network: Network, url: &Url, client: &ApiClient) -> ConnectionResult {
    let result = match timeout(
        Duration::from_secs(CONNECTION_TEST_TIMEOUT_SEC),
        client.get_cached_mixnodes(),
    )
    .await
    {
        Ok(Ok(_)) => {
            log::debug!("Checking: api_url: {network}: {url}: {}", "success".green());
            true
        }
        Ok(Err(e)) => {
            log::debug!(
                "Checking: api_url: {network}: {url}: {}: {e}",
                "failed".red()
            );
            false
        }
        Err(e) => {
            log::debug!(
                "Checking: api_url: {network}: {url}: {}: {e}",
                "failed".red()
            );
            false
        }
    };
    ConnectionResult::Api(network, url.clone(), result)
}

enum ClientForConnectionTest {
    Nymd(Network, Url, Box<NymdClient<QueryNymdClient>>),
    Api(Network, Url, ApiClient),
}

impl ClientForConnectionTest {
    async fn run_connection_check(self) -> ConnectionResult {
        match self {
            ClientForConnectionTest::Nymd(network, ref url, ref client) => {
                test_nymd_connection(network, url, client).await
            }
            ClientForConnectionTest::Api(network, ref url, ref client) => {
                test_api_connection(network, url, client).await
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum UrlType {
    Nymd,
    Api,
}

impl fmt::Display for UrlType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UrlType::Nymd => write!(f, "nymd"),
            UrlType::Api => write!(f, "api"),
        }
    }
}

#[derive(Debug)]
enum ConnectionResult {
    Nymd(Network, Url, bool),
    Api(Network, Url, bool),
}

impl ConnectionResult {
    fn result(&self) -> (&Network, &Url, &bool) {
        match self {
            ConnectionResult::Nymd(network, url, result)
            | ConnectionResult::Api(network, url, result) => (network, url, result),
        }
    }

    fn url_type(&self) -> UrlType {
        match self {
            ConnectionResult::Nymd(..) => UrlType::Nymd,
            ConnectionResult::Api(..) => UrlType::Api,
        }
    }
}

impl fmt::Display for ConnectionResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (network, url, result) = self.result();
        let url_type = self.url_type();
        write!(
            f,
            "{network}: {url}: {url_type}: connection is successful: {result}"
        )
    }
}

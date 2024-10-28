use crate::nyxd::contract_traits::MixnetQueryClient;
use crate::nyxd::error::NyxdError;
use crate::nyxd::Config as ClientConfig;
use crate::{NymApiClient, QueryHttpRpcNyxdClient, ValidatorClientError};
use colored::Colorize;
use core::fmt;
use itertools::Itertools;
use nym_network_defaults::NymNetworkDetails;
use std::collections::HashMap;
use std::hash::BuildHasher;
use std::time::Duration;
use tokio::time::timeout;
use url::Url;

const MAX_URLS_TESTED: usize = 200;
const CONNECTION_TEST_TIMEOUT_SEC: u64 = 2;

/// Run connection tests for all specified nyxd and api urls. These are all run concurrently.
pub async fn run_validator_connection_test<H: BuildHasher + 'static>(
    nyxd_urls: impl Iterator<Item = (NymNetworkDetails, Url)>,
    api_urls: impl Iterator<Item = (NymNetworkDetails, Url)>,
    mixnet_contract_address: HashMap<NymNetworkDetails, cosmrs::AccountId, H>,
) -> (
    HashMap<NymNetworkDetails, Vec<(Url, bool)>>,
    HashMap<NymNetworkDetails, Vec<(Url, bool)>>,
) {
    // Setup all the clients for the connection tests
    let connection_test_clients =
        setup_connection_tests(nyxd_urls, api_urls, mixnet_contract_address);

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
        extract_and_collect_results_into_map(&connection_results, &UrlType::Nyxd),
        extract_and_collect_results_into_map(&connection_results, &UrlType::NymApi),
    )
}

pub async fn test_nyxd_url_connection(
    network: NymNetworkDetails,
    nyxd_url: Url,
    address: cosmrs::AccountId,
) -> Result<bool, ValidatorClientError> {
    let config = ClientConfig::try_from_nym_network_details(&network)
        .expect("failed to create valid nyxd client config");

    let mut nyxd_client = QueryHttpRpcNyxdClient::connect(config, nyxd_url.as_str())?;
    // possibly redundant, but lets just leave it here
    nyxd_client.set_mixnet_contract_address(address);
    match test_nyxd_connection(network, &nyxd_url, &nyxd_client).await {
        ConnectionResult::Nyxd(_, _, res) => Ok(res),
        _ => Ok(false), // ✶ not possible to happens
    }
}

fn setup_connection_tests<H: BuildHasher + 'static>(
    nyxd_urls: impl Iterator<Item = (NymNetworkDetails, Url)>,
    api_urls: impl Iterator<Item = (NymNetworkDetails, Url)>,
    mixnet_contract_address: HashMap<NymNetworkDetails, cosmrs::AccountId, H>,
) -> impl Iterator<Item = ClientForConnectionTest> {
    let nyxd_connection_test_clients = nyxd_urls.filter_map(move |(network, url)| {
        let address = mixnet_contract_address
            .get(&network)
            .expect("No configured contract address")
            .clone();
        let config = ClientConfig::try_from_nym_network_details(&network)
            .expect("failed to create valid nyxd client config");

        if let Ok(mut client) = QueryHttpRpcNyxdClient::connect(config, url.as_str()) {
            // possibly redundant, but lets just leave it here
            client.set_mixnet_contract_address(address);
            Some(ClientForConnectionTest::Nyxd(
                network,
                url,
                Box::new(client),
            ))
        } else {
            None
        }
    });

    let api_connection_test_clients = api_urls.map(|(network, url)| {
        ClientForConnectionTest::Api(network, url.clone(), NymApiClient::new(url))
    });

    nyxd_connection_test_clients.chain(api_connection_test_clients)
}

fn extract_and_collect_results_into_map(
    connection_results: &[ConnectionResult],
    url_type: &UrlType,
) -> HashMap<NymNetworkDetails, Vec<(Url, bool)>> {
    connection_results
        .iter()
        .filter(|c| &c.url_type() == url_type)
        .map(|c| {
            let (network, url, result) = c.result();
            (network.clone(), (url.clone(), *result))
        })
        .into_group_map()
}

async fn test_nyxd_connection(
    network: NymNetworkDetails,
    url: &Url,
    client: &QueryHttpRpcNyxdClient,
) -> ConnectionResult {
    let result = match timeout(
        Duration::from_secs(CONNECTION_TEST_TIMEOUT_SEC),
        client.get_mixnet_contract_version(),
    )
    .await
    {
        Ok(Err(NyxdError::TendermintErrorRpc(e))) => {
            // If we get a tendermint-rpc error, we classify the node as not contactable
            tracing::warn!("Checking: nyxd url: {url}: {}: {}", "failed".red(), e);
            false
        }
        Ok(Err(NyxdError::AbciError { code, log, .. })) => {
            // We accept the mixnet contract not found as ok from a connection standpoint. This happens
            // for example on a pre-launch network.
            tracing::debug!(
                "Checking: nyxd url: {url}: {}, but with abci error: {code}: {log}",
                "success".green()
            );
            code == 18
        }
        Ok(Err(error @ NyxdError::NoContractAddressAvailable(_))) => {
            tracing::warn!("Checking: nyxd url: {url}: {}: {error}", "failed".red());
            false
        }
        Ok(Err(e)) => {
            // For any other error, we're optimistic and just try anyway.
            tracing::warn!(
                "Checking: nyxd_url: {url}: {}, but with error: {e}",
                "success".green()
            );
            true
        }
        Ok(Ok(_)) => {
            tracing::debug!("Checking: nyxd_url: {url}: {}", "success".green());
            true
        }
        Err(e) => {
            tracing::warn!("Checking: nyxd_url: {url}: {}: {e}", "failed".red());
            false
        }
    };
    ConnectionResult::Nyxd(network, url.clone(), result)
}

async fn test_nym_api_connection(
    network: NymNetworkDetails,
    url: &Url,
    client: &NymApiClient,
) -> ConnectionResult {
    let result = match timeout(
        Duration::from_secs(CONNECTION_TEST_TIMEOUT_SEC),
        client.get_cached_mixnodes(),
    )
    .await
    {
        Ok(Ok(_)) => {
            tracing::debug!("Checking: api_url: {url}: {}", "success".green());
            true
        }
        Ok(Err(e)) => {
            tracing::debug!("Checking: api_url: {url}: {}: {e}", "failed".red());
            false
        }
        Err(e) => {
            tracing::debug!("Checking: api_url: {url}: {}: {e}", "failed".red());
            false
        }
    };
    ConnectionResult::Api(network, url.clone(), result)
}

enum ClientForConnectionTest {
    Nyxd(NymNetworkDetails, Url, Box<QueryHttpRpcNyxdClient>),
    Api(NymNetworkDetails, Url, NymApiClient),
}

impl ClientForConnectionTest {
    async fn run_connection_check(self) -> ConnectionResult {
        match self {
            ClientForConnectionTest::Nyxd(network, ref url, ref client) => {
                test_nyxd_connection(network, url, client).await
            }
            ClientForConnectionTest::Api(network, ref url, ref client) => {
                test_nym_api_connection(network, url, client).await
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum UrlType {
    Nyxd,
    NymApi,
}

impl fmt::Display for UrlType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UrlType::Nyxd => write!(f, "nyxd"),
            UrlType::NymApi => write!(f, "api"),
        }
    }
}

#[derive(Debug)]
enum ConnectionResult {
    Nyxd(NymNetworkDetails, Url, bool),
    Api(NymNetworkDetails, Url, bool),
}

impl ConnectionResult {
    fn result(&self) -> (&NymNetworkDetails, &Url, &bool) {
        match self {
            ConnectionResult::Nyxd(network, url, result)
            | ConnectionResult::Api(network, url, result) => (network, url, result),
        }
    }

    fn url_type(&self) -> UrlType {
        match self {
            ConnectionResult::Nyxd(..) => UrlType::Nyxd,
            ConnectionResult::Api(..) => UrlType::NymApi,
        }
    }
}

impl fmt::Display for ConnectionResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (_network, url, result) = self.result();
        let url_type = self.url_type();
        write!(f, "{url}: {url_type}: connection is successful: {result}")
    }
}

use network_defaults::{
    default_api_endpoints, default_nymd_endpoints, DEFAULT_MIXNET_CONTRACT_ADDRESS,
};
use validator_client::nymd::QueryNymdClient;

pub(crate) fn new_nymd_client() -> validator_client::Client<QueryNymdClient> {
    let mixnet_contract = DEFAULT_MIXNET_CONTRACT_ADDRESS.to_string();
    let nymd_url = default_nymd_endpoints()[0].clone();
    let api_url = default_api_endpoints()[0].clone();

    let client_config = validator_client::Config::new(
        nymd_url,
        api_url,
        Some(mixnet_contract.parse().unwrap()),
        None,
        None,
    );

    validator_client::Client::new_query(client_config).expect("Failed to connect to nymd!")
}

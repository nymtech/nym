use mixnet_contract::MixNodeBond;
use validator_client::Config;

pub(crate) async fn retrieve_mixnodes() -> Vec<MixNodeBond> {
    let client = new_validator_client();

    info!("About to retrieve mixnode bonds...");

    let bonds: Vec<MixNodeBond> = match client.get_cached_mix_nodes().await {
        Ok(result) => result,
        Err(e) => panic!("Unable to retrieve mixnode bonds: {:?}", e),
    };
    info!("Fetched {} mixnode bonds", bonds.len());
    bonds
}

// TODO: inject constants
fn new_validator_client() -> validator_client::Client {
    let config = Config::new(vec![crate::VALIDATOR_API.to_string()], crate::CONTRACT);
    validator_client::Client::new(config)
}

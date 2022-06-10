use log::info;
use once_cell::sync::Lazy;
use rand::rngs::OsRng;
use rand::Rng;

use client_core::client::key_manager::KeyManager;
use client_core::config::persistence::key_pathfinder::ClientKeyPathfinder;
use config::NymConfig;

// Generate a random id used for the config, since we need to init a new configuration each time
// due to not being able to reuse gateway registration. This is probably something we should
// improve.
pub static SOCKS5_CONFIG_ID: Lazy<String> = Lazy::new(|| {
    let mut rng = rand::thread_rng();
    format!("{}{:08}", "nym-connect-", rng.gen::<u64>())
});

// TODO: make this configurable from the UI
pub static PROVIDER_ADDRESS: &str = "EWa8DgePKfuWSjqPo6NEdavBK6gpnK4TKb2npi2HWuC2.6PGVT9y83UMGbFrPKDnCvTP2jJjpXYpD87ZpiRsLo1YR@CgQrYP8etksSBf4nALNqp93SHPpgFwEUyTsjBNNLj5WM";

const DEFAULT_ETH_ENDPOINT: &str = "https://rinkeby.infura.io/v3/00000000000000000000000000000000";
const DEFAULT_ETH_PRIVATE_KEY: &str =
    "0000000000000000000000000000000000000000000000000000000000000001";

pub struct Config {}

impl Config {
    pub async fn init() {
        info!("Initialising...");
        init_socks5(PROVIDER_ADDRESS, None).await;
        info!("Configuration saved 🚀");
    }
}

pub async fn init_socks5(provider_address: &str, chosen_gateway_id: Option<&str>) {
    let id: &str = &SOCKS5_CONFIG_ID;
    log::trace!("Creating config for id: {}", id);
    let mut config = nym_socks5::client::config::Config::new(id, provider_address);

    // create identity, encryption and ack keys.
    let mut rng = OsRng;
    let mut key_manager = KeyManager::new(&mut rng);

    info!("Getting gateway details");
    let gateway_details = nym_socks5::commands::init::gateway_details(
        config.get_base().get_validator_api_endpoints(),
        chosen_gateway_id,
    )
    .await;

    info!("Registering with gateway");
    let shared_keys = nym_socks5::commands::init::register_with_gateway(
        &gateway_details,
        key_manager.identity_keypair(),
    )
    .await;

    info!("Setting gateway endpoint");
    config.get_base_mut().with_gateway_endpoint(
        gateway_details.identity_key.to_base58_string(),
        gateway_details.owner.clone(),
        gateway_details.clients_address(),
    );

    info!("Insert gateway shared key");
    key_manager.insert_gateway_shared_key(shared_keys);

    info!("Creating client key path finder");
    let pathfinder = ClientKeyPathfinder::new_from_config(config.get_base());
    key_manager
        .store_keys(&pathfinder)
        .expect("Failed to generated keys");
    info!("Saved all generated keys");

    // As far as I'm aware, these two are not used, they are only set because the socks5 init code
    // requires them for initialising the bandwidth controller.
    config
        .get_base_mut()
        .with_eth_endpoint(DEFAULT_ETH_ENDPOINT);
    config
        .get_base_mut()
        .with_eth_private_key(DEFAULT_ETH_PRIVATE_KEY);

    let config_save_location = config.get_config_file_save_location();
    config
        .save_to_file(None)
        .expect("Failed to save the config file");
    info!("Saved configuration file to {:?}", config_save_location);
    info!("Using gateway: {}", config.get_base().get_gateway_id(),);
    info!("Client configuration completed.\n\n\n");

    nym_socks5::commands::init::show_address(&config);
}

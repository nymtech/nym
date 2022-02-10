use log::info;
use rand::rngs::OsRng;

use client_core::client::key_manager::KeyManager;
use client_core::config::persistence::key_pathfinder::ClientKeyPathfinder;
use config::NymConfig;
#[cfg(not(feature = "coconut"))]
use nym_client::commands::{DEFAULT_ETH_ENDPOINT, DEFAULT_ETH_PRIVATE_KEY};
use nymsphinx::addressing::clients::Recipient;

pub static NATIVE_CLIENT_CONFIG_ID: &str = "hrycyszynvpn";
pub static SOCKS5_CONFIG_ID: &str = "hrycyszynvpn";

// TODO: move to config file
static GATEWAY_ID: &str = "83x9YyNkQ5QEY84ZU6Wmq8XHqfwf9SUtR7g5PAYB1FRY"; // sandbox

pub struct Config {}

impl Config {
  pub async fn init() {
    info!("Initialising...");

    let native_client_address = init_native_client(GATEWAY_ID).await;
    init_socks5(native_client_address, GATEWAY_ID).await;

    info!("Configuration saved 🚀");
  }
}

pub async fn init_native_client(chosen_gateway_id: &str) -> Recipient {
  let mut config = nym_client::client::config::Config::new(NATIVE_CLIENT_CONFIG_ID);

  let mut rng = OsRng;

  // create identity, encryption and ack keys.
  let mut key_manager = KeyManager::new(&mut rng);

  let gateway_details = nym_client::commands::init::gateway_details(
    config.get_base().get_validator_api_endpoints(),
    Some(chosen_gateway_id),
  )
  .await;

  config
    .get_base_mut()
    .with_gateway_id(gateway_details.identity_key.to_base58_string());

  config.get_base_mut().with_testnet_mode(true);
  config
    .get_base_mut()
    .with_eth_endpoint(DEFAULT_ETH_ENDPOINT);
  config
    .get_base_mut()
    .with_eth_private_key(DEFAULT_ETH_PRIVATE_KEY);

  let shared_keys = nym_client::commands::init::register_with_gateway(
    &gateway_details,
    key_manager.identity_keypair(),
  )
  .await;

  config
    .get_base_mut()
    .with_gateway_listener(gateway_details.clients_address());
  key_manager.insert_gateway_shared_key(shared_keys);

  let pathfinder = ClientKeyPathfinder::new_from_config(config.get_base());
  key_manager
    .store_keys(&pathfinder)
    .expect("Failed to generated keys");
  info!("Saved all generated keys");

  let config_save_location = config.get_config_file_save_location();
  config
    .save_to_file(None)
    .expect("Failed to save the config file");
  info!("Saved configuration file to {:?}", config_save_location);
  info!("Using gateway: {}", config.get_base().get_gateway_id(),);
  info!("Client configuration completed.\n\n\n");

  nym_client::commands::init::show_address(&config)
}

pub async fn init_socks5(provider_address: Recipient, chosen_gateway_id: &str) {
  let id = SOCKS5_CONFIG_ID;

  let mut config = nym_socks5::client::config::Config::new(id, &format!("{}", provider_address));

  let mut rng = OsRng;

  // create identity, encryption and ack keys.
  let mut key_manager = KeyManager::new(&mut rng);

  config.get_base_mut().with_testnet_mode(true);
  config
    .get_base_mut()
    .with_eth_endpoint(DEFAULT_ETH_ENDPOINT);
  config
    .get_base_mut()
    .with_eth_private_key(DEFAULT_ETH_PRIVATE_KEY);

  let gateway_details = nym_client::commands::init::gateway_details(
    config.get_base().get_validator_api_endpoints(),
    Some(chosen_gateway_id),
  )
  .await;
  config
    .get_base_mut()
    .with_gateway_id(gateway_details.identity_key.to_base58_string());
  let shared_keys = nym_client::commands::init::register_with_gateway(
    &gateway_details,
    key_manager.identity_keypair(),
  )
  .await;

  config
    .get_base_mut()
    .with_gateway_listener(gateway_details.clients_address());
  key_manager.insert_gateway_shared_key(shared_keys);

  let pathfinder = ClientKeyPathfinder::new_from_config(config.get_base());
  key_manager
    .store_keys(&pathfinder)
    .expect("Failed to generated keys");
  info!("Saved all generated keys");

  let config_save_location = config.get_config_file_save_location();
  config
    .save_to_file(None)
    .expect("Failed to save the config file");
  info!("Saved configuration file to {:?}", config_save_location);
  info!("Using gateway: {}", config.get_base().get_gateway_id(),);
  info!("Client configuration completed.\n\n\n");
}

use log::info;
use rand::rngs::OsRng;

use client_core::client::key_manager::KeyManager;
use client_core::config::persistence::key_pathfinder::ClientKeyPathfinder;
use config::NymConfig;

pub static SOCKS5_CONFIG_ID: &str = "hrycyszynvpn";

// TODO
pub static PROVIDER_ADDRESS: &str = "HL2ufyQ875JNkro17ubKQB4gJyy2ozG8SMrUvHbYXYca.E7iz8hixuWE763h6MdaSnXAXobDqEHuq3Bzvi9XLQhW@BNjYZPxzcJwczXHHgBxCAyVJKxN6LPteDRrKapxWmexv";

// TODO: move to config file
//static GATEWAY_ID: &str = "83x9YyNkQ5QEY84ZU6Wmq8XHqfwf9SUtR7g5PAYB1FRY"; // sandbox

pub struct Config {}

impl Config {
  pub async fn init() {
    info!("Initialising...");

    init_socks5(PROVIDER_ADDRESS, None).await;

    info!("Configuration saved 🚀");
  }
}

pub async fn init_socks5(provider_address: &str, chosen_gateway_id: Option<&str>) {
  let id = SOCKS5_CONFIG_ID;
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

  let config_save_location = config.get_config_file_save_location();
  config
    .save_to_file(None)
    .expect("Failed to save the config file");
  info!("Saved configuration file to {:?}", config_save_location);
  info!("Using gateway: {}", config.get_base().get_gateway_id(),);
  info!("Client configuration completed.\n\n\n");

  nym_socks5::commands::init::show_address(&config);
}

// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{App, Arg, ArgMatches};
use client_core::client::key_manager::KeyManager;
use client_core::config::persistence::key_pathfinder::ClientKeyPathfinder;
use config::NymConfig;
use crypto::asymmetric::{encryption, identity};
use gateway_client::GatewayClient;
use gateway_requests::registration::handshake::SharedKeys;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::addressing::nodes::NodeIdentity;
use rand::{prelude::SliceRandom, rngs::OsRng, thread_rng};
use std::convert::TryInto;
use std::sync::Arc;
use std::time::Duration;
use topology::{filter::VersionFilterable, gateway};
use url::Url;

use crate::client::config::Config;
use crate::commands::override_config;
#[cfg(feature = "eth")]
#[cfg(not(feature = "coconut"))]
use crate::commands::{
    DEFAULT_ETH_ENDPOINT, DEFAULT_ETH_PRIVATE_KEY, ENABLED_CREDENTIALS_MODE_ARG_NAME,
    ETH_ENDPOINT_ARG_NAME, ETH_PRIVATE_KEY_ARG_NAME,
};

pub fn command_args<'a, 'b>() -> clap::App<'a, 'b> {
    let app = App::new("init")
        .about("Initialise a Nym client. Do this first!")
        .arg(Arg::with_name("id")
            .long("id")
            .help("Id of the nym-mixnet-client we want to create config for.")
            .takes_value(true)
            .required(true)
        )
        .arg(Arg::with_name("provider")
            .long("provider")
            .help("Address of the socks5 provider to send messages to.")
            .takes_value(true)
            .required(true)
        )
        .arg(Arg::with_name("gateway")
            .long("gateway")
            .help("Id of the gateway we are going to connect to.")
            .takes_value(true)
        )
        .arg(Arg::with_name("force-register-gateway")
            .long("force-register-gateway")
            .help("Force register gateway. WARNING: this will overwrite any existing keys for the given id, potentially causing loss of access.")
            .takes_value(false)
        )
        .arg(Arg::with_name("validators")
                 .long("validators")
                 .help("Comma separated list of rest endpoints of the validators")
                 .takes_value(true),
        )
        .arg(Arg::with_name("port")
            .short("p")
            .long("port")
            .help("Port for the socket to listen on in all subsequent runs")
            .takes_value(true)
        )
        .arg(Arg::with_name("fastmode")
            .long("fastmode")
            .hidden(true) // this will prevent this flag from being displayed in `--help`
            .help("Mostly debug-related option to increase default traffic rate so that you would not need to modify config post init")
        );
    #[cfg(feature = "eth")]
    #[cfg(not(feature = "coconut"))]
        let app = app
        .arg(
            Arg::with_name(ENABLED_CREDENTIALS_MODE_ARG_NAME)
                .long(ENABLED_CREDENTIALS_MODE_ARG_NAME)
                .help("Set this client to work in a enabled credentials mode that would attempt to use gateway with bandwidth credential requirement. If this value is set, --eth_endpoint and --eth_private_key don't need to be set.")
                .conflicts_with_all(&[ETH_ENDPOINT_ARG_NAME, ETH_PRIVATE_KEY_ARG_NAME])
        )
        .arg(Arg::with_name(ETH_ENDPOINT_ARG_NAME)
            .long(ETH_ENDPOINT_ARG_NAME)
            .help("URL of an Ethereum full node that we want to use for getting bandwidth tokens from ERC20 tokens. If you don't want to set this value, use --testnet-mode instead")
            .takes_value(true)
            .default_value_if(ENABLED_CREDENTIALS_MODE_ARG_NAME, None, DEFAULT_ETH_ENDPOINT)
            .required(true))
        .arg(Arg::with_name(ETH_PRIVATE_KEY_ARG_NAME)
            .long(ETH_PRIVATE_KEY_ARG_NAME)
            .help("Ethereum private key used for obtaining bandwidth tokens from ERC20 tokens. If you don't want to set this value, use --testnet-mode instead")
            .takes_value(true)
            .default_value_if(ENABLED_CREDENTIALS_MODE_ARG_NAME, None, DEFAULT_ETH_PRIVATE_KEY)
            .required(true)
        );

    app
}

// TODO: make this private again after refactoring the config setup
pub async fn register_with_gateway(
    gateway: &gateway::Node,
    our_identity: Arc<identity::KeyPair>,
) -> Arc<SharedKeys> {
    let timeout = Duration::from_millis(1500);
    let mut gateway_client = GatewayClient::new_init(
        gateway.clients_address(),
        gateway.identity_key,
        gateway.owner.clone(),
        our_identity.clone(),
        timeout,
    );
    gateway_client
        .establish_connection()
        .await
        .expect("failed to establish connection with the gateway!");
    gateway_client
        .perform_initial_authentication()
        .await
        .expect("failed to register with the gateway!")
}

// TODO: make this private again after refactoring the config setup
pub async fn gateway_details(
    validator_servers: Vec<Url>,
    chosen_gateway_id: Option<&str>,
) -> gateway::Node {
    let validator_api = validator_servers
        .choose(&mut thread_rng())
        .expect("The list of validator apis is empty");
    let validator_client = validator_client::ApiClient::new(validator_api.clone());

    log::trace!("Fetching list of gateways from: {}", validator_api);
    let gateways = validator_client.get_cached_gateways().await.unwrap();
    let valid_gateways = gateways
        .into_iter()
        .filter_map(|gateway| gateway.try_into().ok())
        .collect::<Vec<gateway::Node>>();

    let filtered_gateways = valid_gateways.filter_by_version(env!("CARGO_PKG_VERSION"));

    // if we have chosen particular gateway - use it, otherwise choose a random one.
    // (remember that in active topology all gateways have at least 100 reputation so should
    // be working correctly)
    if let Some(gateway_id) = chosen_gateway_id {
        filtered_gateways
            .iter()
            .find(|gateway| gateway.identity_key.to_base58_string() == gateway_id)
            .expect(&*format!("no gateway with id {} exists!", gateway_id))
            .clone()
    } else {
        filtered_gateways
            .choose(&mut rand::thread_rng())
            .expect("there are no gateways on the network!")
            .clone()
    }
}

// TODO: make this private again after refactoring the config setup
pub fn show_address(config: &Config) {
    fn load_identity_keys(pathfinder: &ClientKeyPathfinder) -> identity::KeyPair {
        let identity_keypair: identity::KeyPair =
            pemstore::load_keypair(&pemstore::KeyPairPath::new(
                pathfinder.private_identity_key().to_owned(),
                pathfinder.public_identity_key().to_owned(),
            ))
            .expect("Failed to read stored identity key files");
        identity_keypair
    }

    fn load_sphinx_keys(pathfinder: &ClientKeyPathfinder) -> encryption::KeyPair {
        let sphinx_keypair: encryption::KeyPair =
            pemstore::load_keypair(&pemstore::KeyPairPath::new(
                pathfinder.private_encryption_key().to_owned(),
                pathfinder.public_encryption_key().to_owned(),
            ))
            .expect("Failed to read stored sphinx key files");
        sphinx_keypair
    }

    let pathfinder = ClientKeyPathfinder::new_from_config(config.get_base());
    let identity_keypair = load_identity_keys(&pathfinder);
    let sphinx_keypair = load_sphinx_keys(&pathfinder);

    let client_recipient = Recipient::new(
        *identity_keypair.public_key(),
        *sphinx_keypair.public_key(),
        // TODO: below only works under assumption that gateway address == gateway id
        // (which currently is true)
        NodeIdentity::from_base58_string(config.get_base().get_gateway_id()).unwrap(),
    );

    println!("\nThe address of this client is: {}", client_recipient);
}

async fn set_gateway_config(config: &mut Config, chosen_gateway_id: Option<&str>) -> gateway::Node {
    println!("Setting gateway config");
    log::trace!("Chosen gateway: {:?}", chosen_gateway_id);

    // Get the gateway details by querying the validator-api, and using the chosen one if it's
    // among the available ones.
    let gateway_details = gateway_details(
        config.get_base().get_validator_api_endpoints(),
        chosen_gateway_id,
    )
    .await;

    log::trace!("Used gateway: {}", gateway_details);

    config
        .get_base_mut()
        .with_gateway_endpoint(gateway_details.clone().into());
    gateway_details
}

async fn register_and_store_gateway_keys(gateway_details: gateway::Node, config: &Config) {
    println!("Registering gateway");

    let mut rng = OsRng;
    let mut key_manager = KeyManager::new(&mut rng);
    let shared_keys = register_with_gateway(&gateway_details, key_manager.identity_keypair()).await;
    key_manager.insert_gateway_shared_key(shared_keys);

    let pathfinder = ClientKeyPathfinder::new_from_config(config.get_base());
    key_manager
        .store_keys(&pathfinder)
        .expect("Failed to generated keys");
    println!("Saved all generated keys");
}

pub async fn execute(matches: ArgMatches<'static>) {
    println!("Initialising client...");

    let id = matches.value_of("id").unwrap(); // required for now
    let provider_address = matches.value_of("provider").unwrap();

    let already_init = Config::default_config_file_path(Some(id)).exists();
    if already_init {
        println!(
            "SOCKS5 client \"{}\" was already initialised before! \
            Config information will be overwritten (but keys will be kept)!",
            id
        );
    }

    // Usually you only register with the gateway on the first init, however you can force
    // re-registering if wanted.
    let should_force_register = matches.is_present("force-register-gateway");

    // If the client was already initialized, don't generate new keys and don't re-register with
    // the gateway (because this would create a new shared key).
    // Unless the user really wants to.
    let will_register_gateway = !already_init || should_force_register;

    // Attempt to use a user-provided gateway, if possible
    let is_gateway_provided = matches.value_of("gateway").is_some();

    let mut config = Config::new(id, provider_address);

    // TODO: ideally that should be the last thing that's being done to config.
    // However, we are later further overriding it with gateway id
    config = override_config(config, &matches);
    if matches.is_present("fastmode") {
        config.get_base_mut().set_high_default_traffic_volume();
    }

    if will_register_gateway {
        // Create identity, encryption and ack keys.
        let gateway_details = set_gateway_config(&mut config, matches.value_of("gateway")).await;
        register_and_store_gateway_keys(gateway_details, &config).await;
    } else if is_gateway_provided {
        // Just set the config, don't register or create any keys
        set_gateway_config(&mut config, matches.value_of("gateway")).await;
    } else {
        // Read the existing config to reuse the gateway configuration
        println!("Not registering gateway, will reuse existing config and keys");
        if let Ok(existing_config) = Config::load_from_file(Some(id)) {
            config
                .get_base_mut()
                .with_gateway_endpoint(existing_config.get_base().get_gateway_endpoint().clone());
        } else {
            log::warn!(
                "Existing configuration found, but enable to load gateway details. \
                Proceeding anyway."
            );
        };
    }

    let config_save_location = config.get_config_file_save_location();
    config
        .save_to_file(None)
        .expect("Failed to save the config file");
    println!("Saved configuration file to {:?}", config_save_location);
    println!("Using gateway: {}", config.get_base().get_gateway_id());
    log::debug!("Gateway id: {}", config.get_base().get_gateway_id());
    log::debug!("Gateway owner: {}", config.get_base().get_gateway_owner());
    log::debug!(
        "Gateway listener: {}",
        config.get_base().get_gateway_listener()
    );
    println!("Client configuration completed.");

    show_address(&config);
}

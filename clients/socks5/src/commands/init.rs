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
    DEFAULT_ETH_ENDPOINT, DEFAULT_ETH_PRIVATE_KEY, ETH_ENDPOINT_ARG_NAME, ETH_PRIVATE_KEY_ARG_NAME,
    TESTNET_MODE_ARG_NAME,
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
            Arg::with_name(TESTNET_MODE_ARG_NAME)
                .long(TESTNET_MODE_ARG_NAME)
                .help("Set this client to work in a testnet mode that would attempt to use gateway without bandwidth credential requirement. If this value is set, --eth_endpoint and --eth_private_key don't need to be set.")
                .conflicts_with_all(&[ETH_ENDPOINT_ARG_NAME, ETH_PRIVATE_KEY_ARG_NAME])
        )
        .arg(Arg::with_name(ETH_ENDPOINT_ARG_NAME)
            .long(ETH_ENDPOINT_ARG_NAME)
            .help("URL of an Ethereum full node that we want to use for getting bandwidth tokens from ERC20 tokens. If you don't want to set this value, use --testnet-mode instead")
            .takes_value(true)
            .default_value_if(TESTNET_MODE_ARG_NAME, None, DEFAULT_ETH_ENDPOINT)
            .required(true))
        .arg(Arg::with_name(ETH_PRIVATE_KEY_ARG_NAME)
            .long(ETH_PRIVATE_KEY_ARG_NAME)
            .help("Ethereum private key used for obtaining bandwidth tokens from ERC20 tokens. If you don't want to set this value, use --testnet-mode instead")
            .takes_value(true)
            .default_value_if(TESTNET_MODE_ARG_NAME, None, DEFAULT_ETH_PRIVATE_KEY)
            .required(true)
        );

    app
}

async fn register_with_gateway(
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

async fn gateway_details(
    validator_servers: Vec<Url>,
    chosen_gateway_id: Option<&str>,
) -> gateway::Node {
    let validator_api = validator_servers
        .choose(&mut thread_rng())
        .expect("The list of validator apis is empty");
    let validator_client = validator_client::ApiClient::new(validator_api.clone());

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

fn show_address(config: &Config) {
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

pub async fn execute(matches: ArgMatches<'static>) {
    println!("Initialising client...");

    let id = matches.value_of("id").unwrap(); // required for now
    let provider_address = matches.value_of("provider").unwrap();

    let already_init = if Config::default_config_file_path(Some(id)).exists() {
        println!("Socks5 client \"{}\" was already initialised before! Config information will be overwritten (but keys will be kept)!", id);
        true
    } else {
        false
    };

    let mut config = Config::new(id, provider_address);

    let mut rng = OsRng;

    // TODO: ideally that should be the last thing that's being done to config.
    // However, we are later further overriding it with gateway id
    config = override_config(config, &matches);
    if matches.is_present("fastmode") {
        config.get_base_mut().set_high_default_traffic_volume();
    }

    // if client was already initialised, don't generate new keys, not re-register with gateway
    // (because this would create new shared key)
    if !already_init {
        // create identity, encryption and ack keys.
        let mut key_manager = KeyManager::new(&mut rng);

        let chosen_gateway_id = matches.value_of("gateway");

        let gateway_details = gateway_details(
            config.get_base().get_validator_api_endpoints(),
            chosen_gateway_id,
        )
        .await;
        let shared_keys =
            register_with_gateway(&gateway_details, key_manager.identity_keypair()).await;

        config.get_base_mut().with_gateway_endpoint(
            gateway_details.identity_key.to_base58_string(),
            gateway_details.owner.clone(),
            gateway_details.clients_address(),
        );
        key_manager.insert_gateway_shared_key(shared_keys);

        let pathfinder = ClientKeyPathfinder::new_from_config(config.get_base());
        key_manager
            .store_keys(&pathfinder)
            .expect("Failed to generated keys");
        println!("Saved all generated keys");
    }

    let config_save_location = config.get_config_file_save_location();
    config
        .save_to_file(None)
        .expect("Failed to save the config file");
    println!("Saved configuration file to {:?}", config_save_location);
    println!("Using gateway: {}", config.get_base().get_gateway_id(),);
    println!("Client configuration completed.\n\n\n");

    show_address(&config);
}

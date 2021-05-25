// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::client::config::Config;
use crate::commands::override_config;
use clap::{App, Arg, ArgMatches};
use client_core::client::key_manager::KeyManager;
use client_core::config::persistence::key_pathfinder::ClientKeyPathfinder;
use config::NymConfig;
use crypto::asymmetric::{encryption, identity};
use gateway_client::GatewayClient;
use gateway_requests::registration::handshake::SharedKeys;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::addressing::nodes::NodeIdentity;
use rand::{prelude::SliceRandom, rngs::OsRng};
use std::convert::TryInto;
use std::sync::Arc;
use std::time::Duration;
use topology::{filter::VersionFilterable, gateway};

pub fn command_args<'a, 'b>() -> clap::App<'a, 'b> {
    App::new("init")
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
        .arg(Arg::with_name("mixnet-contract")
                 .long("mixnet-contract")
                 .help("Address of the validator contract managing the network")
                 .takes_value(true),
        )
        .arg(Arg::with_name("port")
            .short("p")
            .long("port")
            .help("Port for the socket to listen on in all subsequent runs")
            .takes_value(true)
        )
        .arg(Arg::with_name("vpn-mode")
            .long("vpn-mode")
            .help("Set the vpn mode of the client")
            .long_help(
                r#" 
                    Special mode of the system such that all messages are sent as soon as they are received
                    and no cover traffic is generated. If set all message delays are set to 0 and overwriting
                    'Debug' values will have no effect.
                "#
            )
        )
        .arg(Arg::with_name("fastmode")
            .long("fastmode")
            .hidden(true) // this will prevent this flag from being displayed in `--help`
            .help("Mostly debug-related option to increase default traffic rate so that you would not need to modify config post init")
        )
}

async fn register_with_gateway(
    gateway: &gateway::Node,
    our_identity: Arc<identity::KeyPair>,
) -> SharedKeys {
    let timeout = Duration::from_millis(1500);
    let mut gateway_client = GatewayClient::new_init(
        gateway.client_listener.clone(),
        gateway.identity_key,
        our_identity.clone(),
        timeout,
    );
    gateway_client
        .establish_connection()
        .await
        .expect("failed to establish connection with the gateway!");
    gateway_client
        .register()
        .await
        .expect("failed to register with the gateway!")
}

async fn gateway_details(
    validator_servers: Vec<String>,
    mixnet_contract: &str,
    chosen_gateway_id: Option<&str>,
) -> gateway::Node {
    let validator_client_config = validator_client::Config::new(validator_servers, mixnet_contract);
    let mut validator_client = validator_client::Client::new(validator_client_config);

    let gateways = validator_client.get_gateways().await.unwrap();
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

pub fn execute(matches: &ArgMatches) {
    println!("Initialising client...");

    let id = matches.value_of("id").unwrap(); // required for now
    let provider_address = matches.value_of("provider").unwrap();

    let already_init = if Config::default_config_file_path(id).exists() {
        println!("Socks5 client \"{}\" was already initialised before! Config information will be overwritten (but keys will be kept)!", id);
        true
    } else {
        false
    };

    let mut config = Config::new(id, provider_address);

    let mut rng = OsRng;

    // TODO: ideally that should be the last thing that's being done to config.
    // However, we are later further overriding it with gateway id
    config = override_config(config, matches);
    if matches.is_present("fastmode") {
        config.get_base_mut().set_high_default_traffic_volume();
    }

    // if client was already initialised, don't generate new keys, not re-register with gateway
    // (because this would create new shared key)
    if !already_init {
        // create identity, encryption and ack keys.
        let mut key_manager = KeyManager::new(&mut rng);

        let chosen_gateway_id = matches.value_of("gateway");

        let registration_fut = async {
            let gate_details = gateway_details(
                config.get_base().get_validator_rest_endpoints(),
                &config.get_base().get_validator_mixnet_contract_address(),
                chosen_gateway_id,
            )
            .await;
            config
                .get_base_mut()
                .with_gateway_id(gate_details.identity_key.to_base58_string());
            let shared_keys =
                register_with_gateway(&gate_details, key_manager.identity_keypair()).await;
            (shared_keys, gate_details.client_listener)
        };

        // TODO: is there perhaps a way to make it work without having to spawn entire runtime?
        let rt = tokio::runtime::Runtime::new().unwrap();
        let (shared_keys, gateway_listener) = rt.block_on(registration_fut);
        config
            .get_base_mut()
            .with_gateway_listener(gateway_listener);
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

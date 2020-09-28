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

use crate::built_info;
use crate::client::config::Config;
use crate::commands::override_config;
use clap::{App, Arg, ArgMatches};
use client_core::client::key_manager::KeyManager;
use client_core::config::persistence::key_pathfinder::ClientKeyPathfinder;
use config::NymConfig;
use crypto::asymmetric::identity;
use directory_client::DirectoryClient;
use gateway_client::GatewayClient;
use gateway_requests::registration::handshake::SharedKeys;
use rand::{prelude::SliceRandom, rngs::OsRng};
use std::convert::TryInto;
use std::sync::Arc;
use std::time::Duration;
use topology::{gateway, NymTopology};

const GOOD_GATEWAYS: [&str; 2] = [
    "D6YaMzLSY7mANtSQRKXsmMZpqgqiVkeiagKM4V4oFPFr",
    "5nrYxPR8gt2Gzo2BbHtsGf66KAEQY91WmM1eW78EphNy",
];

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
        .arg(Arg::with_name("directory")
            .long("directory")
            .help("Address of the directory server the client is getting topology from")
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
        )
}

async fn register_with_gateway(
    gateway: &gateway::Node,
    our_identity: Arc<identity::KeyPair>,
) -> SharedKeys {
    let timeout = Duration::from_millis(1500);
    let mut gateway_client = GatewayClient::new_init(
        url::Url::parse(&gateway.client_listener).unwrap(),
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

async fn gateway_details(directory_server: &str, gateway_id: &str) -> gateway::Node {
    let directory_client_config = directory_client::Config::new(directory_server.to_string());
    let directory_client = directory_client::Client::new(directory_client_config);
    let topology = directory_client.get_topology().await.unwrap();
    let nym_topology: NymTopology = topology.try_into().expect("Invalid topology data!");
    let version_filtered_topology = nym_topology.filter_system_version(built_info::PKG_VERSION);

    version_filtered_topology
        .gateways()
        .iter()
        .find(|gateway| gateway.identity_key.to_base58_string() == gateway_id)
        .expect(&*format!("no gateway with id {} exists!", gateway_id))
        .clone()
}

fn select_gateway(arg: Option<&str>) -> &str {
    if let Some(gateway_id) = arg {
        gateway_id
    } else {
        // TODO1: this should only be done on testnet
        // TODO2: it should probably check if chosen gateway is actually online
        GOOD_GATEWAYS.choose(&mut rand::thread_rng()).unwrap()
    }
}

pub fn execute(matches: &ArgMatches) {
    println!("Initialising client...");

    let id = matches.value_of("id").unwrap(); // required for now
    let provider_address = matches.value_of("provider").unwrap();

    let mut config = Config::new(id, provider_address);
    let mut rng = OsRng;

    // TODO: ideally that should be the last thing that's being done to config.
    // However, we are later further overriding it with gateway id
    config = override_config(config, matches);
    if matches.is_present("fastmode") {
        config.get_base_mut().set_high_default_traffic_volume();
    }

    // create identity, encryption and ack keys.
    let mut key_manager = KeyManager::new(&mut rng);

    let gateway_id = select_gateway(matches.value_of("gateway"));
    config.get_base_mut().with_gateway_id(gateway_id);
    println!("Using gateway {}", gateway_id);

    let registration_fut = async {
        let gate_details =
            gateway_details(&config.get_base().get_directory_server(), gateway_id).await;
        let shared_keys =
            register_with_gateway(&gate_details, key_manager.identity_keypair()).await;
        (shared_keys, gate_details.client_listener)
    };

    // TODO: is there perhaps a way to make it work without having to spawn entire runtime?
    let mut rt = tokio::runtime::Runtime::new().unwrap();
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

    let config_save_location = config.get_config_file_save_location();
    config
        .save_to_file(None)
        .expect("Failed to save the config file");
    println!("Saved configuration file to {:?}", config_save_location);
    println!("Using gateway: {}", config.get_base().get_gateway_id(),);
    println!("Client configuration completed.\n\n\n")
}

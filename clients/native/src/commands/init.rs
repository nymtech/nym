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
use crate::client::key_manager::KeyManager;
use crate::commands::override_config;
use crate::config::persistence::pathfinder::ClientPathfinder;
use clap::{App, Arg, ArgMatches};
use config::NymConfig;
use crypto::asymmetric::identity;
use directory_client::DirectoryClient;
use gateway_client::GatewayClient;
use gateway_requests::registration::handshake::SharedKey;
use rand::rngs::OsRng;
use std::convert::TryInto;
use std::sync::Arc;
use std::time::Duration;
use topology::{gateway, NymTopology};

pub fn command_args<'a, 'b>() -> clap::App<'a, 'b> {
    App::new("init")
        .about("Initialise a Nym client. Do this first!")
        .arg(Arg::with_name("id")
            .long("id")
            .help("Id of the nym-mixnet-client we want to create config for.")
            .takes_value(true)
            .required(true)
        )
        .arg(Arg::with_name("gateway")
            .long("gateway")
            .help("Id of the gateway we have preference to connect to. If left empty, a random gateway will be chosen.")
            .takes_value(true)
        )
        .arg(Arg::with_name("directory")
            .long("directory")
            .help("Address of the directory server the client is getting topology from")
            .takes_value(true),
        )
        .arg(Arg::with_name("disable-socket")
            .long("disable-socket")
            .help("Whether to not start the websocket")
        )
        .arg(Arg::with_name("port")
            .short("p")
            .long("port")
            .help("Port for the socket (if applicable) to listen on in all subsequent runs")
            .takes_value(true)
        )
        .arg(Arg::with_name("fastmode")
            .long("fastmode")
            .hidden(true) // this will prevent this flag from being displayed in `--help`
            .help("Mostly debug-related option to increase default traffic rate so that you would not need to modify config post init")
        )
}

async fn try_gateway_registration(
    gateways: &Vec<gateway::Node>,
    our_identity: Arc<identity::KeyPair>,
) -> Option<(String, String, SharedKey)> {
    let timeout = Duration::from_millis(1500);
    for gateway in gateways {
        let mut gateway_client = GatewayClient::new_init(
            url::Url::parse(&gateway.client_listener).unwrap(),
            gateway.identity_key.clone(),
            our_identity.clone(),
            timeout,
        );
        if let Ok(_) = gateway_client.establish_connection().await {
            if let Ok(shared_key) = gateway_client.register().await {
                if let Err(err) = gateway_client.close_connection().await {
                    eprintln!("Error while closing connection to the gateway! - {:?}", err);
                    continue;
                } else {
                    return Some((
                        gateway.identity_key.to_base58_string(),
                        gateway.client_listener.clone(),
                        shared_key,
                    ));
                }
            }
        }
    }
    None
}

async fn choose_gateway(
    directory_server: String,
    our_identity: Arc<identity::KeyPair>,
) -> (String, String, SharedKey) {
    let directory_client_config = directory_client::Config::new(directory_server.clone());
    let directory_client = directory_client::Client::new(directory_client_config);
    let topology = directory_client.get_topology().await.unwrap();
    let nym_topology: NymTopology = topology.try_into().expect("Invalid topology data!");

    let version_filtered_topology = nym_topology.filter_system_version(built_info::PKG_VERSION);
    // don't care about health of the networks as mixes can go up and down any time,
    // but DO care about gateways
    let gateways = version_filtered_topology.gateways();

    // try to perform registration so that we wouldn't need to do it at startup
    // + at the same time we'll know if we can actually talk with that gateway
    let registration_result = try_gateway_registration(gateways, our_identity).await;
    match registration_result {
        None => {
            // while technically there's no issue client-side, it will be impossible to execute
            // `nym-client run` as no gateway is available so it might be best to not finalize
            // the init and rely on users trying to init another time?
            panic!(
                "Currently there are no valid gateways available on the network ({}). \
                 Please try to run `init` again at later time or change your directory server",
                directory_server
            )
        }
        Some((gateway_id, gateway_listener, shared_key)) => {
            (gateway_id, gateway_listener, shared_key)
        }
    }
}

async fn get_gateway_listener(directory_server: String, gateway_identity: &str) -> Option<String> {
    let directory_client_config = directory_client::Config::new(directory_server);
    let directory_client = directory_client::Client::new(directory_client_config);
    let topology = directory_client.get_topology().await.unwrap();

    // technically we don't need to do conversion here, but let's be consistent
    let nym_topology: NymTopology = topology.try_into().ok()?;
    let gateways = nym_topology.gateways();

    for gateway in gateways {
        if gateway.identity_key.to_base58_string() == gateway_identity {
            return Some(gateway.client_listener.clone());
        }
    }
    None
}

pub fn execute(matches: &ArgMatches) {
    println!("Initialising client...");

    let id = matches.value_of("id").unwrap(); // required for now
    let mut config = crate::config::Config::new(id);
    let mut rng = OsRng;

    config = override_config(config, matches);
    if matches.is_present("fastmode") {
        config = config.set_high_default_traffic_volume();
    }

    // create identity, encryption and ack keys.
    let mut key_manager = KeyManager::new(&mut rng);

    // if there is no gateway chosen, get a random-ish one from the topology
    if config.get_gateway_id().is_empty() {
        // TODO: is there perhaps a way to make it work without having to spawn entire runtime?
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        let (gateway_id, gateway_listener, shared_key) = rt.block_on(choose_gateway(
            config.get_directory_server(),
            key_manager.identity_keypair(),
        ));

        config = config
            .with_gateway_id(gateway_id)
            .with_gateway_listener(gateway_listener);

        key_manager.insert_gateway_shared_key(shared_key)
    }

    // we specified our gateway but don't know its physical address
    if config.get_gateway_listener().is_empty() {
        // TODO: is there perhaps a way to make it work without having to spawn entire runtime?
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        let gateway_listener = rt
            .block_on(get_gateway_listener(
                config.get_directory_server(),
                &config.get_gateway_id(),
            ))
            .expect("No gateway with provided id exists!");

        config = config.with_gateway_listener(gateway_listener);
    }

    let pathfinder = ClientPathfinder::new_from_config(&config);
    key_manager
        .store_keys(&pathfinder)
        .expect("Failed to generated keys");
    println!("Saved all generated keys");

    let config_save_location = config.get_config_file_save_location();
    config
        .save_to_file(None)
        .expect("Failed to save the config file");
    println!("Saved configuration file to {:?}", config_save_location);

    println!(
        "Unless overridden in all `nym-client run` we will be talking to the following gateway: {}...",
        config.get_gateway_id(),
    );

    println!("Client configuration completed.\n\n\n")
}

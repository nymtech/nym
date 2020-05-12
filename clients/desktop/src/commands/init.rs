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
use crate::commands::override_config;
use crate::config::persistence::pathfinder::ClientPathfinder;
use clap::{App, Arg, ArgMatches};
use config::NymConfig;
use crypto::identity::MixIdentityKeyPair;
use directory_client::presence::Topology;
use futures::channel::mpsc;
use gateway_client::GatewayClient;
use gateway_requests::AuthToken;
use nymsphinx::DestinationAddressBytes;
use pemstore::pemstore::PemStore;
use std::time::Duration;
use topology::gateway::Node;
use topology::NymTopology;

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
}

async fn try_gateway_registration(
    gateways: Vec<Node>,
    our_address: DestinationAddressBytes,
) -> Option<(String, AuthToken)> {
    // TODO: having to do something like this suggests that perhaps GatewayClient's constructor
    // could be improved
    let (sphinx_tx, _) = mpsc::unbounded();
    let timeout = Duration::from_millis(1500);
    for gateway in gateways {
        let mut gateway_client = GatewayClient::new(
            url::Url::parse(&gateway.client_listener).unwrap(),
            our_address.clone(),
            None,
            sphinx_tx.clone(),
            timeout,
        );
        if gateway_client.establish_connection().await.is_ok() {
            if let Ok(token) = gateway_client.register().await {
                return Some((gateway.pub_key, token));
            }
        }
    }
    None
}

async fn choose_gateway(
    directory_server: String,
    our_address: DestinationAddressBytes,
) -> (String, AuthToken) {
    // TODO: once we change to graph topology this here will need to be updated!
    let topology = Topology::new(directory_server.clone()).await;
    let version_filtered_topology = topology.filter_system_version(built_info::PKG_VERSION);
    // don't care about health of the networks as mixes can go up and down any time,
    // but DO care about gateways
    let gateways = version_filtered_topology.gateways();

    // try to perform registration so that we wouldn't need to do it at startup
    // + at the same time we'll know if we can actually talk with that gateway
    let registration_result = try_gateway_registration(gateways, our_address).await;
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
        Some((gateway_id, auth_token)) => (gateway_id, auth_token),
    }
}

pub fn execute(matches: &ArgMatches) {
    println!("Initialising client...");

    let id = matches.value_of("id").unwrap(); // required for now
    let mut config = crate::config::Config::new(id);

    config = override_config(config, matches);

    let mix_identity_keys = MixIdentityKeyPair::new();

    // if there is no gateway chosen, get a random-ish one from the topology
    if config.get_gateway_id().is_empty() {
        let our_address = mix_identity_keys.public_key().derive_address();
        // TODO: is there perhaps a way to make it work without having to spawn entire runtime?
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        let (gateway_id, auth_token) =
            rt.block_on(choose_gateway(config.get_directory_server(), our_address));

        // TODO: this isn't really a gateway, but gateway, yet another change to make
        config = config
            .with_gateway_id(gateway_id)
            .with_gateway_auth_token(auth_token);
    }

    let pathfinder = ClientPathfinder::new_from_config(&config);
    let pem_store = PemStore::new(pathfinder);
    pem_store
        .write_identity(mix_identity_keys)
        .expect("Failed to save identity keys");
    println!("Saved mixnet identity keypair");

    let config_save_location = config.get_config_file_save_location();
    config
        .save_to_file(None)
        .expect("Failed to save the config file");
    println!("Saved configuration file to {:?}", config_save_location);

    println!(
        "Unless overridden in all `nym-client run` we will be talking to the following gateway: {}...",
        config.get_gateway_id(),
    );
    if config.get_gateway_auth_token().is_some() {
        println!(
            "using optional AuthToken: {:?}",
            config.get_gateway_auth_token().unwrap()
        )
    }
    println!("Client configuration completed.\n\n\n")
}

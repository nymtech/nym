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

use crate::commands::override_config;
use crate::config::persistence::pathfinder::GatewayPathfinder;
use crate::config::Config;
use crate::node::Gateway;
use clap::{App, Arg, ArgMatches};
use config::NymConfig;
use crypto::encryption;
use directory_client::requests::presence_topology_get::PresenceTopologyGetRequester;
use directory_client::DirectoryClient;
use pemstore::pemstore::PemStore;

pub fn command_args<'a, 'b>() -> clap::App<'a, 'b> {
    App::new("run")
        .about("Starts the gateway")
        .arg(
            Arg::with_name("id")
                .long("id")
                .help("Id of the gateway we want to run")
                .takes_value(true)
                .required(true),
        )
        // the rest of arguments are optional, they are used to override settings in config file
        .arg(
            Arg::with_name("location")
                .long("location")
                .help("Optional geographical location of this gateway")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("config")
                .long("config")
                .help("Custom path to the nym gateway configuration file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("mix-host")
                .long("mix-host")
                .help("The custom host on which the gateway will be running for receiving sphinx packets")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("mix-port")
                .long("mix-port")
                .help("The port on which the gateway will be listening for sphinx packets")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("clients-host")
                .long("clients-host")
                .help("The custom host on which the gateway will be running for receiving clients gateway-requests")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("clients-port")
                .long("clients-port")
                .help("The port on which the gateway will be listening for clients gateway-requests")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("mix-announce-host")
                .long("mix-announce-host")
                .help("The host that will be reported to the directory server")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("mix-announce-port")
                .long("mix-announce-port")
                .help("The port that will be reported to the directory server")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("clients-announce-host")
                .long("clients-announce-host")
                .help("The host that will be reported to the directory server")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("clients-announce-port")
                .long("clients-announce-port")
                .help("The port that will be reported to the directory server")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("inboxes")
                .long("inboxes")
                .help("Directory with inboxes where all packets for the clients are stored")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("clients-ledger")
                .long("clients-ledger")
                .help("Ledger file containing registered clients")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("directory")
                .long("directory")
                .help("Address of the directory server the gateway is sending presence data to")
                .takes_value(true),
        )
}

fn show_binding_warning(address: String) {
    println!("\n##### NOTE #####");
    println!(
        "\nYou are trying to bind to {} - you might not be accessible to other nodes\n\
         You can ignore this warning if you're running setup on a local network \n\
         or have set a custom 'announce-host'",
        address
    );
    println!("\n\n");
}

fn special_addresses() -> Vec<&'static str> {
    vec!["localhost", "127.0.0.1", "0.0.0.0", "::1", "[::1]"]
}

fn load_sphinx_keys(config_file: &Config) -> encryption::KeyPair {
    let sphinx_keypair = PemStore::new(GatewayPathfinder::new_from_config(&config_file))
        .read_encryption()
        .expect("Failed to read stored sphinx key files");
    println!(
        "Public key: {}\n",
        sphinx_keypair.public_key().to_base58_string()
    );
    sphinx_keypair
}

fn check_if_same_ip_gateway_exists(
    directory_server: String,
    announced_mix_host: String,
    announced_clients_host: String,
) -> Option<String> {
    let directory_client_cfg = directory_client::Config::new(directory_server);
    let topology = directory_client::Client::new(directory_client_cfg)
        .presence_topology
        .get()
        .expect("Failed to retrieve network topology");

    let existing_gateways = topology.gateway_nodes;
    existing_gateways
        .iter()
        .find(|node| {
            node.mixnet_listener == announced_mix_host
                || node.client_listener == announced_clients_host
        })
        .map(|node| node.pub_key.clone())
}

pub fn execute(matches: &ArgMatches) {
    let id = matches.value_of("id").unwrap();

    println!("Starting gateway {}...", id);

    let mut config =
        Config::load_from_file(matches.value_of("config").map(|path| path.into()), Some(id))
            .expect("Failed to load config file");

    config = override_config(config, matches);

    if let Some(duplicate_gateway_key) = check_if_same_ip_gateway_exists(
        config.get_presence_directory_server(),
        config.get_mix_announce_address(),
        config.get_clients_announce_address(),
    ) {
        println!(
            "Our announce-host is identical to an existing node's announce-host! (its key is {:?}",
            duplicate_gateway_key
        );
        return;
    }

    let sphinx_keypair = load_sphinx_keys(&config);

    let mix_listening_ip_string = config.get_mix_listening_address().ip().to_string();
    if special_addresses().contains(&mix_listening_ip_string.as_ref()) {
        show_binding_warning(mix_listening_ip_string);
    }

    let clients_listening_ip_string = config.get_clients_listening_address().ip().to_string();
    if special_addresses().contains(&clients_listening_ip_string.as_ref()) {
        show_binding_warning(clients_listening_ip_string);
    }

    println!(
        "Directory server [presence]: {}",
        config.get_presence_directory_server()
    );

    println!(
        "Listening for incoming sphinx packets on {}",
        config.get_mix_listening_address()
    );
    println!(
        "Announcing the following socket address for sphinx packets: {}",
        config.get_mix_announce_address()
    );

    println!(
        "Listening for incoming clients packets on {}",
        config.get_clients_listening_address()
    );
    println!(
        "Announcing the following socket address for clients packets: {}",
        config.get_clients_announce_address()
    );

    println!(
        "Inboxes directory is: {:?}",
        config.get_clients_inboxes_dir()
    );

    println!(
        "Clients ledger is stored at: {:?}",
        config.get_clients_ledger_path()
    );

    Gateway::new(config, sphinx_keypair).run();
}

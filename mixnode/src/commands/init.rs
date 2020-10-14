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
use crate::config::persistence::pathfinder::MixNodePathfinder;
use crate::config::Config;
use clap::{App, Arg, ArgMatches};
use config::NymConfig;
use crypto::asymmetric::encryption;
use directory_client::DirectoryClient;
use log::*;
use nymsphinx::params::DEFAULT_NUM_MIX_HOPS;
use std::convert::TryInto;
use std::process;
use tokio::runtime::Runtime;
use topology::NymTopology;

pub fn command_args<'a, 'b>() -> clap::App<'a, 'b> {
    App::new("init")
        .about("Initialise the mixnode")
        .arg(
            Arg::with_name("id")
                .long("id")
                .help("Id of the nym-mixnode we want to create config for.")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("location")
                .long("location")
                .help("Optional geographical location of this node")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("layer")
                .long("layer")
                .help("The mixnet layer of this particular node")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("host")
                .long("host")
                .help("The host on which the mixnode will be running")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("port")
                .long("port")
                .help("The port on which the mixnode will be listening")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("announce-host")
                .long("announce-host")
                .help("The custom host that will be reported to the directory server")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("announce-port")
                .long("announce-port")
                .help("The custom port that will be reported to the directory server")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("directory")
                .long("directory")
                .help("Address of the directory server the node is sending presence and metrics to")
                .takes_value(true),
        )
}

fn show_incentives_url() {
    println!("\n##### NOTE #####");
    println!(
        "\nIf you would like to join our testnet incentives program, please visit https://nymtech.net/incentives"
    );
    println!("\n\n");
}

async fn choose_layer(matches: &ArgMatches<'_>, directory_server: String) -> u64 {
    let max_layer = DEFAULT_NUM_MIX_HOPS;
    if let Some(layer) = matches.value_of("layer").map(|layer| layer.parse::<u64>()) {
        if let Err(err) = layer {
            // if layer was overridden, it must be parsable
            panic!("Invalid layer value provided - {:?}", err);
        }
        let layer = layer.unwrap();
        if layer <= max_layer as u64 && layer > 0 {
            return layer;
        }
    }

    let directory_client_config = directory_client::Config::new(directory_server);
    let directory_client = directory_client::Client::new(directory_client_config);
    let topology: NymTopology = directory_client
        .get_topology()
        .await
        .expect("failed to obtain initial network topology!")
        .try_into()
        .unwrap();

    let mut lowest_layer = (0, usize::max_value());

    for layer in 1..=max_layer {
        let nodes_count = topology
            .mixes()
            .get(&layer)
            .map(|layer_mixes| layer_mixes.len())
            .unwrap_or_else(|| 0);
        trace!("There are {} nodes on layer {}", nodes_count, layer);
        if nodes_count < lowest_layer.1 {
            lowest_layer.0 = layer;
            lowest_layer.1 = nodes_count;
        }
    }

    lowest_layer.0 as u64
}

pub fn execute(matches: &ArgMatches) {
    // TODO: this should probably be made implicit by slapping `#[tokio::main]` on our main method
    // and then removing runtime from mixnode itself in `run`
    let mut rt = Runtime::new().unwrap();
    rt.block_on(async {
        let id = matches.value_of("id").unwrap();
        println!("Initialising mixnode {}...", id);

        if Config::default_config_file_path(id).exists() {
            eprintln!("Mixnode {} was already initialised before! If you wanted to upgrade your node to most recent version, try `upgrade` command instead!", id);
            process::exit(1);
        }

        let mut config = Config::new(id);
        config = override_config(config, matches);
        let layer = choose_layer(matches, config.get_presence_directory_server()).await;
        // TODO: I really don't like how we override config and are presumably done with it
        // only to change it here
        config = config.with_layer(layer);
        debug!("Choosing layer {}", config.get_layer());

        let sphinx_keys = encryption::KeyPair::new();
        let pathfinder = MixNodePathfinder::new_from_config(&config);
        pemstore::store_keypair(
            &sphinx_keys,
            &pemstore::KeyPairPath::new(
                pathfinder.private_encryption_key().to_owned(),
                pathfinder.public_encryption_key().to_owned(),
            ),
        )
        .expect("Failed to save sphinx keys");
        println!("Saved mixnet sphinx keypair");
        let config_save_location = config.get_config_file_save_location();
        config
            .save_to_file(None)
            .expect("Failed to save the config file");
        println!("Saved configuration file to {:?}", config_save_location);
        println!("Mixnode configuration completed.\n\n\n");
        show_incentives_url();
    })
}

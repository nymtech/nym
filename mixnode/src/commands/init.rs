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
use clap::{App, Arg, ArgMatches};
use config::NymConfig;
use crypto::asymmetric::encryption;
use rand::{rngs::OsRng, Rng};

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

pub fn execute(matches: &ArgMatches) {
    let id = matches.value_of("id").unwrap();
    println!("Initialising mixnode {}...", id);

    let layer = match matches.value_of("layer") {
        Some(layer) => layer.parse().unwrap(),
        None => {
            let mut rng = OsRng;
            rng.gen_range(1, 4)
        }
    };

    let mut config = crate::config::Config::new(id, layer);

    config = override_config(config, matches);

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
}

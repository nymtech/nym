// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::*;
use crate::config::persistence::pathfinder::MixNodePathfinder;
use crate::config::Config;
use crate::node::MixNode;
use clap::{App, Arg, ArgMatches};
use config::NymConfig;
use crypto::asymmetric::{encryption, identity};
use tokio::runtime::Runtime;

pub fn command_args<'a, 'b>() -> clap::App<'a, 'b> {
    App::new("init")
        .about("Initialise the mixnode")
        .arg(
            Arg::with_name(ID_ARG_NAME)
                .long(ID_ARG_NAME)
                .help("Id of the nym-mixnode we want to create config for.")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(HOST_ARG_NAME)
                .long(HOST_ARG_NAME)
                .help("The host on which the mixnode will be running")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(MIX_PORT_ARG_NAME)
                .long(MIX_PORT_ARG_NAME)
                .help("The port on which the mixnode will be listening for mix packets")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(VERLOC_PORT_ARG_NAME)
                .long(VERLOC_PORT_ARG_NAME)
                .help("The port on which the mixnode will be listening for verloc packets")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(HTTP_API_PORT_ARG_NAME)
                .long(HTTP_API_PORT_ARG_NAME)
                .help("The port on which the mixnode will be listening for http requests")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(ANNOUNCE_HOST_ARG_NAME)
                .long(ANNOUNCE_HOST_ARG_NAME)
                .help("The custom host that will be reported to the directory server")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(VALIDATORS_ARG_NAME)
                .long(VALIDATORS_ARG_NAME)
                .help("Comma separated list of rest endpoints of the validators")
                .takes_value(true),
        )
}

pub fn execute(matches: &ArgMatches) {
    // TODO: this should probably be made implicit by slapping `#[tokio::main]` on our main method
    // and then removing runtime from mixnode itself in `run`
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let id = matches.value_of(ID_ARG_NAME).unwrap();
        println!("Initialising mixnode {}...", id);

        let already_init = if Config::default_config_file_path(Some(id)).exists() {
            println!("Mixnode \"{}\" was already initialised before! Config information will be overwritten (but keys will be kept)!", id);
            true
        } else {
            false
        };

        let mut config = Config::new(id);
        config = override_config(config, matches);

        // if node was already initialised, don't generate new keys
        if !already_init {
            let mut rng = rand::rngs::OsRng;

            let identity_keys = identity::KeyPair::new(&mut rng);
            let sphinx_keys = encryption::KeyPair::new(&mut rng);
            let pathfinder = MixNodePathfinder::new_from_config(&config);
            pemstore::store_keypair(
                &identity_keys,
                &pemstore::KeyPairPath::new(
                    pathfinder.private_identity_key().to_owned(),
                    pathfinder.public_identity_key().to_owned(),
                ),
            )
                .expect("Failed to save identity keys");

            pemstore::store_keypair(
                &sphinx_keys,
                &pemstore::KeyPairPath::new(
                    pathfinder.private_encryption_key().to_owned(),
                    pathfinder.public_encryption_key().to_owned(),
                ),
            )
                .expect("Failed to save sphinx keys");

            println!("Saved mixnet identity and sphinx keypairs");
        }

        let config_save_location = config.get_config_file_save_location();
        config
            .save_to_file(None)
            .expect("Failed to save the config file");
        println!("Saved configuration file to {:?}", config_save_location);
        println!("Mixnode configuration completed.\n\n\n");

        MixNode::new(config).print_node_details()
    })
}

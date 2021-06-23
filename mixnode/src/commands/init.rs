// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::*;
use crate::config::persistence::pathfinder::MixNodePathfinder;
use crate::config::Config;
use clap::{App, Arg, ArgMatches};
use config::NymConfig;
use crypto::asymmetric::{encryption, identity};
use log::debug;
use nymsphinx::params::DEFAULT_NUM_MIX_HOPS;
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
            Arg::with_name(LAYER_ARG_NAME)
                .long(LAYER_ARG_NAME)
                .help("The mixnet layer of this particular node")
                .takes_value(true),
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
                .help("The port on which the mixnode will be listening")
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
        .arg(
            Arg::with_name(CONTRACT_ARG_NAME)
                .long(CONTRACT_ARG_NAME)
                .help("Address of the validator contract managing the network")
                .takes_value(true),
        )
}

async fn choose_layer(
    matches: &ArgMatches<'_>,
    validator_servers: Vec<String>,
    mixnet_contract: String,
) -> u64 {
    let max_layer = DEFAULT_NUM_MIX_HOPS;
    if let Some(layer) = matches
        .value_of(LAYER_ARG_NAME)
        .map(|layer| layer.parse::<u64>())
    {
        if let Err(err) = layer {
            // if layer was overridden, it must be parsable
            panic!("Invalid layer value provided - {:?}", err);
        }
        let layer = layer.unwrap();
        if layer <= max_layer as u64 && layer > 0 {
            return layer;
        }
    }

    let validator_client_config = validator_client::Config::new(validator_servers, mixnet_contract);
    let mut validator_client = validator_client::Client::new(validator_client_config);

    let layer_distribution = validator_client
        .get_layer_distribution()
        .await
        .expect("failed to obtain layer distribution of the testnet!");

    if layer_distribution.layer1 < layer_distribution.layer2
        && layer_distribution.layer1 < layer_distribution.layer3
    {
        1
    } else if layer_distribution.layer2 < layer_distribution.layer1
        && layer_distribution.layer2 < layer_distribution.layer3
    {
        2
    } else {
        3
    }
}

fn show_bonding_info(config: &Config) {
    fn load_identity_keys(pathfinder: &MixNodePathfinder) -> identity::KeyPair {
        let identity_keypair: identity::KeyPair =
            pemstore::load_keypair(&pemstore::KeyPairPath::new(
                pathfinder.private_identity_key().to_owned(),
                pathfinder.public_identity_key().to_owned(),
            ))
            .expect("Failed to read stored identity key files");
        identity_keypair
    }

    fn load_sphinx_keys(pathfinder: &MixNodePathfinder) -> encryption::KeyPair {
        let sphinx_keypair: encryption::KeyPair =
            pemstore::load_keypair(&pemstore::KeyPairPath::new(
                pathfinder.private_encryption_key().to_owned(),
                pathfinder.public_encryption_key().to_owned(),
            ))
            .expect("Failed to read stored sphinx key files");
        sphinx_keypair
    }

    let pathfinder = MixNodePathfinder::new_from_config(config);
    let identity_keypair = load_identity_keys(&pathfinder);
    let sphinx_keypair = load_sphinx_keys(&pathfinder);

    println!(
        "\nTo bond your mixnode you will need to provide the following:
    Identity key: {}
    Sphinx key: {}
    Address: {}
    Mix port: {}
    Layer: {}
    Version: {}
    ",
        identity_keypair.public_key().to_base58_string(),
        sphinx_keypair.public_key().to_base58_string(),
        config.get_announce_address(),
        config.get_mix_port(),
        config.get_layer(),
        config.get_version(),
    );
}

pub fn execute(matches: &ArgMatches) {
    // TODO: this should probably be made implicit by slapping `#[tokio::main]` on our main method
    // and then removing runtime from mixnode itself in `run`
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let id = matches.value_of(ID_ARG_NAME).unwrap();
        println!("Initialising mixnode {}...", id);

        let already_init = if Config::default_config_file_path(id).exists() {
            println!("Mixnode \"{}\" was already initialised before! Config information will be overwritten (but keys will be kept)!", id);
            true
        } else {
            false
        };

        let mut config = Config::new(id);
        config = override_config(config, matches);
        let layer = choose_layer(matches, config.get_validator_rest_endpoints(), config.get_validator_mixnet_contract_address()).await;
        // TODO: I really don't like how we override config and are presumably done with it
        // only to change it here
        config = config.with_layer(layer);
        debug!("Choosing layer {}", config.get_layer());

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

        show_bonding_info(&config)
    })
}

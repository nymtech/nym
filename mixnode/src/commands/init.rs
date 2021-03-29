// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::override_config;
use crate::config::persistence::pathfinder::MixNodePathfinder;
use crate::config::Config;
use clap::{App, Arg, ArgMatches};
use config::NymConfig;
use crypto::asymmetric::{encryption, identity};
use log::*;
use nymsphinx::params::DEFAULT_NUM_MIX_HOPS;
use std::collections::HashMap;
use tokio::runtime::Runtime;

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
            Arg::with_name("validator")
                .long("validator")
                .help("REST endpoint of the validator the node is registering presence with")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("mixnet-contract")
                .long("mixnet-contract")
                .help("Address of the validator contract managing the network")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("metrics-server")
                .long("metrics-server")
                .help("Server to which the node is sending all metrics data")
                .takes_value(true),
        )
}

async fn choose_layer(
    matches: &ArgMatches<'_>,
    validator_server: String,
    mixnet_contract: String,
) -> u64 {
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

    let validator_client_config =
        validator_client_rest::Config::new(validator_server, mixnet_contract);
    let validator_client = validator_client_rest::Client::new(validator_client_config);

    let mixnodes = validator_client
        .get_mix_nodes()
        .await
        .expect("failed to obtain initial network mixnodes");

    let mut nodes_distribution = HashMap::new();
    // initialise with 0 for each possible layer
    for layer in 1..=max_layer {
        nodes_distribution.insert(layer as u64, 0);
    }

    for node in mixnodes {
        if node.mix_node.layer < 1 || node.mix_node.layer > max_layer as u64 {
            warn!(
                "one of bonded mixnodes is on invalid layer {}",
                node.mix_node.layer
            );
            continue;
        }

        *nodes_distribution.entry(node.mix_node.layer).or_insert(0) += 1;
    }

    // this can't be None as the hashmap is guaranteed to be non-empty since we initialised it
    // with zeroes for each possible layer
    let layer_with_fewest = nodes_distribution
        .iter()
        .min_by(|a, b| a.1.cmp(&b.1))
        .map(|(k, _v)| k)
        .unwrap();

    *layer_with_fewest
}

pub fn execute(matches: &ArgMatches) {
    // TODO: this should probably be made implicit by slapping `#[tokio::main]` on our main method
    // and then removing runtime from mixnode itself in `run`
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let id = matches.value_of("id").unwrap();
        println!("Initialising mixnode {}...", id);

        let already_init = if Config::default_config_file_path(id).exists() {
            println!("Mixnode \"{}\" was already initialised before! Config information will be overwritten (but keys will be kept)!", id);
            true
        } else {
            false
        };

        let mut config = Config::new(id);
        config = override_config(config, matches);
        let layer = choose_layer(matches, config.get_validator_rest_endpoint(), config.get_validator_mixnet_contract_address()).await;
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
    })
}

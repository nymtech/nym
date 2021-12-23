// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::*;
use crate::config::persistence::pathfinder::GatewayPathfinder;
use crate::config::Config;
use crate::node::Gateway;
use clap::{App, Arg, ArgMatches};
use config::NymConfig;
use crypto::asymmetric::{encryption, identity};

pub fn command_args<'a, 'b>() -> clap::App<'a, 'b> {
    let app = App::new("init")
        .about("Initialise the gateway")
        .arg(
            Arg::with_name(ID_ARG_NAME)
                .long(ID_ARG_NAME)
                .help("Id of the gateway we want to create config for.")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(HOST_ARG_NAME)
                .long(HOST_ARG_NAME)
                .help("The custom host on which the gateway will be running for receiving sphinx packets")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(MIX_PORT_ARG_NAME)
                .long(MIX_PORT_ARG_NAME)
                .help("The port on which the gateway will be listening for sphinx packets")
                .takes_value(true)
        )
        .arg(
            Arg::with_name(CLIENTS_PORT_ARG_NAME)
                .long(CLIENTS_PORT_ARG_NAME)
                .help("The port on which the gateway will be listening for clients gateway-requests")
                .takes_value(true)
        )
        .arg(
            Arg::with_name(ANNOUNCE_HOST_ARG_NAME)
                .long(ANNOUNCE_HOST_ARG_NAME)
                .help("The host that will be reported to the directory server")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(DATASTORE_PATH)
                .long(DATASTORE_PATH)
                .help("Path to sqlite database containing all gateway persistent data")
                .takes_value(true)
        )
        .arg(
            Arg::with_name(VALIDATOR_APIS_ARG_NAME)
                .long(VALIDATOR_APIS_ARG_NAME)
                .help("Comma separated list of endpoints of the validators APIs")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(WALLET_ADDRESS)
            .long(WALLET_ADDRESS)
            .help("The wallet address you will use to bond this gateway, e.g. nymt1z9egw0knv47nmur0p8vk4rcx59h9gg4zuxrrr9")
            .takes_value(true)
            .required(true)
        );

    #[cfg(feature = "eth")]
    #[cfg(not(feature = "coconut"))]
    let app = app
        .arg(
            Arg::with_name(TESTNET_MODE_ARG_NAME)
                .long(TESTNET_MODE_ARG_NAME)
                .help("Set this gateway to work in a testnet mode that would allow clients to bypass bandwidth credential requirement")
        )
        .arg(Arg::with_name(ETH_ENDPOINT)
            .long(ETH_ENDPOINT)
            .help("URL of an Ethereum full node that we want to use for getting bandwidth tokens from ERC20 tokens")
            .takes_value(true)
            .required(true))
        .arg(Arg::with_name(VALIDATORS_ARG_NAME)
            .long(VALIDATORS_ARG_NAME)
            .help("Comma separated list of endpoints of the validator")
            .takes_value(true))
        .arg(Arg::with_name(COSMOS_MNEMONIC)
            .long(COSMOS_MNEMONIC)
            .help("Cosmos wallet mnemonic")
            .takes_value(true)
            .required(true));

    app
}

pub async fn execute(matches: ArgMatches<'static>) {
    let id = matches.value_of(ID_ARG_NAME).unwrap();
    println!("Initialising gateway {}...", id);

    let already_init = if Config::default_config_file_path(Some(id)).exists() {
        println!("Gateway \"{}\" was already initialised before! Config information will be overwritten (but keys will be kept)!", id);
        true
    } else {
        false
    };

    let mut config = Config::new(id);

    config = override_config(config, &matches);

    // if gateway was already initialised, don't generate new keys
    if !already_init {
        let mut rng = rand::rngs::OsRng;

        let identity_keys = identity::KeyPair::new(&mut rng);
        let sphinx_keys = encryption::KeyPair::new(&mut rng);
        let pathfinder = GatewayPathfinder::new_from_config(&config);
        pemstore::store_keypair(
            &sphinx_keys,
            &pemstore::KeyPairPath::new(
                pathfinder.private_encryption_key().to_owned(),
                pathfinder.public_encryption_key().to_owned(),
            ),
        )
        .expect("Failed to save sphinx keys");

        pemstore::store_keypair(
            &identity_keys,
            &pemstore::KeyPairPath::new(
                pathfinder.private_identity_key().to_owned(),
                pathfinder.public_identity_key().to_owned(),
            ),
        )
        .expect("Failed to save identity keys");

        println!("Saved identity and mixnet sphinx keypairs");
    }

    let config_save_location = config.get_config_file_save_location();
    config
        .save_to_file(None)
        .expect("Failed to save the config file");
    println!("Saved configuration file to {:?}", config_save_location);
    println!("Gateway configuration completed.\n\n\n");

    Gateway::new(config).await.print_node_details()
}

// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::{persistence::pathfinder::GatewayPathfinder, Config};
use clap::{App, Arg, ArgMatches};
use config::NymConfig;
use crypto::asymmetric::identity;
use log::*;
use tokio::runtime::Runtime;

pub fn command_args<'a, 'b>() -> clap::App<'a, 'b> {
    App::new("unregister").about("Unregister the gateway").arg(
        Arg::with_name("id")
            .long("id")
            .help("Id of the nym-gateway we want to explicitly unregister")
            .takes_value(true)
            .required(true),
    )
}

fn load_identity_keys(pathfinder: &GatewayPathfinder) -> identity::KeyPair {
    pemstore::load_keypair(&pemstore::KeyPairPath::new(
        pathfinder.private_identity_key().to_owned(),
        pathfinder.public_identity_key().to_owned(),
    ))
    .expect("Failed to read stored identity key files")
}

pub fn execute(matches: &ArgMatches) {
    // TODO: this should probably be made implicit by slapping `#[tokio::main]` on our main method
    // and then removing runtime from gateway itself in `run`
    let mut rt = Runtime::new().unwrap();
    rt.block_on(async {
        let id = matches.value_of("id").unwrap();

        println!("Attempting to unregister gateway {}...", id);

        let config = match Config::load_from_file(id) {
            Ok(cfg) => cfg,
            Err(err) => {
                error!("Failed to load config for {}. Are you sure you have run provided correct id? (Error was: {})", id, err);
                return;
            }
        };

        // we need to load identity keys to be able to grab node's public key
        let pathfinder = GatewayPathfinder::new_from_config(&config);
        let identity_keypair = load_identity_keys(&pathfinder);

        // now attempt to unregister
        let validator_client_config = validator_client::Config::new(config.get_validator_rest_endpoint());
        let validator_client = validator_client::Client::new(validator_client_config);

        match validator_client.unregister_node(&*identity_keypair.public_key().to_base58_string()).await {
            Err(err) => error!("failed to unregister node '{}'. Error: {:?}", id, err),
            Ok(_) => info!("managed to successfully unregister node '{}'!", id)
        }
    })
}

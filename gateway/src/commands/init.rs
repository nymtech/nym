// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    commands::{override_config, OverrideConfig},
    config::{persistence::pathfinder::GatewayPathfinder, Config},
    node::Gateway,
};
use clap::Args;
use config::NymConfig;
use crypto::asymmetric::{encryption, identity};

#[derive(Args, Clone)]
pub struct Init {
    /// Id of the gateway we want to create config for
    #[clap(long)]
    id: String,

    /// The custom host on which the gateway will be running for receiving sphinx packets
    #[clap(long)]
    host: String,

    /// The wallet address you will use to bond this gateway, e.g. nymt1z9egw0knv47nmur0p8vk4rcx59h9gg4zuxrrr9
    #[clap(long)]
    wallet_address: String,

    /// The port on which the gateway will be listening for sphinx packets
    #[clap(long)]
    mix_port: Option<u16>,

    /// The port on which the gateway will be listening for clients gateway-requests
    #[clap(long)]
    clients_port: Option<u16>,

    /// The host that will be reported to the directory server
    #[clap(long)]
    announce_host: Option<String>,

    /// Path to sqlite database containing all gateway persistent data
    #[clap(long)]
    datastore: Option<String>,

    /// Comma separated list of endpoints of the validators APIs
    #[clap(long)]
    validator_apis: Option<String>,

    /// Set this gateway to work in a testnet mode that would allow clients to bypass bandwidth credential requirement
    #[cfg(all(feature = "eth", not(feature = "coconut")))]
    #[clap(long)]
    testnet_mode: bool,

    /// URL of an Ethereum full node that we want to use for getting bandwidth tokens from ERC20 tokens
    #[cfg(all(feature = "eth", not(feature = "coconut")))]
    #[clap(long)]
    eth_endpoint: String,

    /// Comma separated list of endpoints of the validator
    #[cfg(all(feature = "eth", not(feature = "coconut")))]
    #[clap(long)]
    validators: Option<String>,

    /// Cosmos wallet mnemonic
    #[cfg(all(feature = "eth", not(feature = "coconut")))]
    #[clap(long)]
    mnemonic: String,
}

impl From<Init> for OverrideConfig {
    fn from(init_config: Init) -> Self {
        OverrideConfig {
            id: init_config.id,
            host: Some(init_config.host),
            wallet_address: Some(init_config.wallet_address),
            mix_port: init_config.mix_port,
            clients_port: init_config.clients_port,
            datastore: init_config.datastore,
            announce_host: init_config.announce_host,
            validator_apis: init_config.validator_apis,

            #[cfg(all(feature = "eth", not(feature = "coconut")))]
            testnet_mode: init_config.testnet_mode,

            #[cfg(all(feature = "eth", not(feature = "coconut")))]
            eth_endpoint: Some(init_config.eth_endpoint),

            #[cfg(all(feature = "eth", not(feature = "coconut")))]
            validators: init_config.validators,

            #[cfg(all(feature = "eth", not(feature = "coconut")))]
            mnemonic: Some(init_config.mnemonic),
        }
    }
}

pub async fn execute(args: &Init) {
    let override_config_fields = OverrideConfig::from(args.clone());
    let id = &override_config_fields.id;
    println!("Initialising gateway {}...", id);

    let already_init = if Config::default_config_file_path(Some(id)).exists() {
        println!("Gateway \"{}\" was already initialised before! Config information will be overwritten (but keys will be kept)!", id);
        true
    } else {
        false
    };

    let mut config = Config::new(id);

    config = override_config(config, override_config_fields);

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

    Gateway::new(config).await.print_node_details();
}

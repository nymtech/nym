// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::OverrideConfig;
use crate::config::Config;
use crate::node::MixNode;
use crate::{commands::override_config, config::persistence::pathfinder::MixNodePathfinder};
use clap::Args;
use config::NymConfig;
use crypto::asymmetric::{encryption, identity};
use std::net::IpAddr;
use validator_client::nyxd;

#[derive(Args, Clone)]
pub(crate) struct Init {
    /// Id of the mixnode we want to create config for
    #[clap(long)]
    id: String,

    /// The host on which the mixnode will be running
    #[clap(long)]
    host: IpAddr,

    /// The wallet address you will use to bond this mixnode, e.g. nymt1z9egw0knv47nmur0p8vk4rcx59h9gg4zuxrrr9
    #[clap(long)]
    wallet_address: nyxd::AccountId,

    /// The port on which the mixnode will be listening for mix packets
    #[clap(long)]
    mix_port: Option<u16>,

    /// The port on which the mixnode will be listening for verloc packets
    #[clap(long)]
    verloc_port: Option<u16>,

    /// The port on which the mixnode will be listening for http requests
    #[clap(long)]
    http_api_port: Option<u16>,

    /// The custom host that will be reported to the directory server
    #[clap(long)]
    announce_host: Option<String>,

    /// Comma separated list of nym-api endpoints of the validators
    // the alias here is included for backwards compatibility (1.1.4 and before)
    #[clap(long, alias = "validators", value_delimiter = ',')]
    nym_apis: Option<Vec<url::Url>>,
}

impl From<Init> for OverrideConfig {
    fn from(init_config: Init) -> Self {
        OverrideConfig {
            id: init_config.id,
            host: Some(init_config.host),
            wallet_address: Some(init_config.wallet_address),
            mix_port: init_config.mix_port,
            verloc_port: init_config.verloc_port,
            http_api_port: init_config.http_api_port,
            announce_host: init_config.announce_host,
            nym_apis: init_config.nym_apis,
        }
    }
}

pub(crate) fn execute(args: &Init) {
    let override_config_fields = OverrideConfig::from(args.clone());
    let id = &override_config_fields.id;
    println!("Initialising mixnode {}...", id);

    let already_init = if Config::default_config_file_path(Some(id)).exists() {
        println!("Mixnode \"{}\" was already initialised before! Config information will be overwritten (but keys will be kept)!", id);
        true
    } else {
        false
    };

    let mut config = Config::new(id);
    config = override_config(config, override_config_fields);

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
}

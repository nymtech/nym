// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::OverrideConfig;
use crate::commands::override_config;
use crate::config::{
    default_config_directory, default_config_filepath, default_data_directory, Config,
};
use crate::node::MixNode;
use clap::Args;
use nym_bin_common::output_format::OutputFormat;
use nym_crypto::asymmetric::{encryption, identity};
use std::net::IpAddr;
use std::{fs, io};

#[derive(Args, Clone)]
pub(crate) struct Init {
    /// Id of the mixnode we want to create config for
    #[clap(long)]
    id: String,

    /// The host on which the mixnode will be running
    #[clap(long)]
    host: IpAddr,

    /// The port on which the mixnode will be listening for mix packets
    #[clap(long)]
    mix_port: Option<u16>,

    /// The port on which the mixnode will be listening for verloc packets
    #[clap(long)]
    verloc_port: Option<u16>,

    /// The port on which the mixnode will be listening for http requests
    #[clap(long)]
    http_api_port: Option<u16>,

    /// Comma separated list of nym-api endpoints of the validators
    // the alias here is included for backwards compatibility (1.1.4 and before)
    #[clap(long, alias = "validators", value_delimiter = ',')]
    nym_apis: Option<Vec<url::Url>>,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

impl From<Init> for OverrideConfig {
    fn from(init_config: Init) -> Self {
        OverrideConfig {
            id: init_config.id,
            host: Some(init_config.host),
            mix_port: init_config.mix_port,
            verloc_port: init_config.verloc_port,
            http_api_port: init_config.http_api_port,
            nym_apis: init_config.nym_apis,
        }
    }
}

fn init_paths(id: &str) -> io::Result<()> {
    fs::create_dir_all(default_data_directory(id))?;
    fs::create_dir_all(default_config_directory(id))
}

pub(crate) fn execute(args: &Init) -> anyhow::Result<()> {
    let override_config_fields = OverrideConfig::from(args.clone());
    let id = override_config_fields.id.clone();
    eprintln!("Initialising mixnode {id}...");

    let already_init = if default_config_filepath(&id).exists() {
        eprintln!("Mixnode \"{id}\" was already initialised before! Config information will be overwritten (but keys will be kept)!");
        true
    } else {
        init_paths(&id).expect("failed to initialise storage paths");
        false
    };

    let mut config = Config::new(&id);
    config = override_config(config, override_config_fields);

    // if node was already initialised, don't generate new keys
    if !already_init {
        let mut rng = rand::rngs::OsRng;

        let identity_keys = identity::KeyPair::new(&mut rng);
        let sphinx_keys = encryption::KeyPair::new(&mut rng);

        nym_pemstore::store_keypair(
            &identity_keys,
            &nym_pemstore::KeyPairPath::new(
                config.storage_paths.private_identity_key(),
                config.storage_paths.public_identity_key(),
            ),
        )
        .expect("Failed to save identity keys");

        nym_pemstore::store_keypair(
            &sphinx_keys,
            &nym_pemstore::KeyPairPath::new(
                config.storage_paths.private_encryption_key(),
                config.storage_paths.public_encryption_key(),
            ),
        )
        .expect("Failed to save sphinx keys");
        eprintln!("Saved mixnet identity and sphinx keypairs");
    }

    let config_save_location = config.default_location();
    config.save_to_default_location().unwrap_or_else(|_| {
        panic!(
            "Failed to save the config file to {}",
            config_save_location.display()
        )
    });
    eprintln!(
        "Saved configuration file to {}",
        config_save_location.display()
    );
    eprintln!("Mixnode configuration completed.\n\n\n");

    MixNode::new(config)?.print_node_details(args.output);
    Ok(())
}

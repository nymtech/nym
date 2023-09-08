// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::helpers::initialise_local_network_requester;
use crate::config::{default_config_directory, default_config_filepath, default_data_directory};
use crate::{commands::helpers::OverrideConfig, config::Config, OutputFormat};
use clap::Args;
use nym_crypto::asymmetric::{encryption, identity};
use std::error::Error;
use std::net::IpAddr;
use std::path::PathBuf;
use std::{fs, io};

#[derive(Args, Clone)]
pub struct Init {
    /// Id of the gateway we want to create config for
    #[clap(long)]
    id: String,

    /// The custom host on which the gateway will be running for receiving sphinx packets
    #[clap(long)]
    host: IpAddr,

    /// The port on which the gateway will be listening for sphinx packets
    #[clap(long)]
    mix_port: Option<u16>,

    /// The port on which the gateway will be listening for clients gateway-requests
    #[clap(long)]
    clients_port: Option<u16>,

    /// Path to sqlite database containing all gateway persistent data
    #[clap(long)]
    datastore: Option<PathBuf>,

    /// Comma separated list of endpoints of nym APIs
    #[clap(long, alias = "validator_apis", value_delimiter = ',')]
    // the alias here is included for backwards compatibility (1.1.4 and before)
    nym_apis: Option<Vec<url::Url>>,

    /// Comma separated list of endpoints of the validator
    #[clap(
        long,
        alias = "validators",
        alias = "nyxd_validators",
        value_delimiter = ',',
        hide = true
    )]
    // the alias here is included for backwards compatibility (1.1.4 and before)
    nyxd_urls: Option<Vec<url::Url>>,

    /// Cosmos wallet mnemonic needed for double spending protection
    #[clap(long)]
    mnemonic: Option<bip39::Mnemonic>,

    /// Set this gateway to work only with coconut credentials; that would disallow clients to
    /// bypass bandwidth credential requirement
    #[clap(long, hide = true)]
    only_coconut_credentials: Option<bool>,

    /// Enable/disable gateway anonymized statistics that get sent to a statistics aggregator server
    #[clap(long)]
    enabled_statistics: Option<bool>,

    /// URL where a statistics aggregator is running. The default value is a Nym aggregator server
    #[clap(long)]
    statistics_service_url: Option<url::Url>,

    /// Allows this gateway to run an embedded network requester for minimal network overhead
    #[clap(long)]
    with_network_requester: Option<bool>,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

impl From<Init> for OverrideConfig {
    fn from(init_config: Init) -> Self {
        OverrideConfig {
            host: Some(init_config.host),
            mix_port: init_config.mix_port,
            clients_port: init_config.clients_port,
            datastore: init_config.datastore,
            nym_apis: init_config.nym_apis,
            mnemonic: init_config.mnemonic,

            enabled_statistics: init_config.enabled_statistics,
            statistics_service_url: init_config.statistics_service_url,

            nyxd_urls: init_config.nyxd_urls,
            only_coconut_credentials: init_config.only_coconut_credentials,
            with_network_requester: init_config.with_network_requester,
        }
    }
}

fn init_paths(id: &str) -> io::Result<()> {
    fs::create_dir_all(default_data_directory(id))?;
    fs::create_dir_all(default_config_directory(id))
}

pub async fn execute(args: Init) -> Result<(), Box<dyn Error + Send + Sync>> {
    eprintln!("Initialising gateway {}...", args.id);
    let output = args.output;

    let already_init = if default_config_filepath(&args.id).exists() {
        eprintln!(
            "Gateway \"{}\" was already initialised before! Config information will be \
            overwritten (but keys will be kept)!",
            args.id
        );
        true
    } else {
        init_paths(&args.id)?;
        false
    };

    // Initialising the config structure is just overriding a default constructed one
    let fresh_config = Config::new(&args.id);
    let config = OverrideConfig::from(args).do_override(fresh_config)?;

    // if gateway was already initialised, don't generate new keys, et al.
    let nr_details = if !already_init {
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

        let mut details = None;
        if config.network_requester.enabled {
            details = Some(
                initialise_local_network_requester(&config, *identity_keys.public_key()).await?,
            );
        }

        eprintln!("Saved identity and mixnet sphinx keypairs");
        details
    } else {
        None
    };

    let config_save_location = config.default_location();
    config
        .save_to_default_location()
        .expect("Failed to save the config file");
    eprintln!(
        "Saved configuration file to {}",
        config_save_location.display()
    );
    eprintln!("Gateway configuration completed.\n\n\n");

    crate::node::create_gateway(config, None)
        .await?
        .print_node_details(output);
    Ok(())
}

#[cfg(test)]
mod tests {
    use nym_network_defaults::var_names::BECH32_PREFIX;

    use crate::node::{storage::InMemStorage, Gateway};

    use super::*;

    #[tokio::test]
    async fn create_gateway_with_in_mem_storage() {
        let args = Init {
            id: "foo-id".to_string(),
            host: "1.1.1.1".parse().unwrap(),
            mix_port: Some(42),
            clients_port: Some(43),
            datastore: Some("/foo-datastore".parse().unwrap()),
            nym_apis: None,
            mnemonic: None,
            statistics_service_url: None,
            enabled_statistics: None,
            nyxd_urls: None,
            only_coconut_credentials: None,
            output: Default::default(),
            with_network_requester: None,
        };
        std::env::set_var(BECH32_PREFIX, "n");

        let fresh_config = Config::new(&args.id);
        let config = OverrideConfig::from(args)
            .do_override(fresh_config)
            .unwrap();

        let (identity_keys, sphinx_keys) = {
            let mut rng = rand::rngs::OsRng;
            (
                identity::KeyPair::new(&mut rng),
                encryption::KeyPair::new(&mut rng),
            )
        };

        // The test is really if this instantiates with InMemStorage without panics
        let _gateway = Gateway::new_from_keys_and_storage(
            config,
            None,
            identity_keys,
            sphinx_keys,
            InMemStorage,
        )
        .await;
    }
}

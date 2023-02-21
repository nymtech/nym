// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    commands::{override_config, OverrideConfig},
    config::{persistence::pathfinder::GatewayPathfinder, Config},
    OutputFormat,
};
use clap::Args;
use config::NymConfig;
use nym_crypto::asymmetric::{encryption, identity};
use std::error::Error;
use std::net::IpAddr;
use std::path::PathBuf;
use validator_client::nyxd;

#[derive(Args, Clone)]
pub struct Init {
    /// Id of the gateway we want to create config for
    #[clap(long)]
    id: String,

    /// The custom host on which the gateway will be running for receiving sphinx packets
    #[clap(long)]
    host: IpAddr,

    /// The wallet address you will use to bond this gateway, e.g. nymt1z9egw0knv47nmur0p8vk4rcx59h9gg4zuxrrr9
    #[clap(long)]
    wallet_address: nyxd::AccountId,

    /// The port on which the gateway will be listening for sphinx packets
    #[clap(long)]
    mix_port: Option<u16>,

    /// The port on which the gateway will be listening for clients gateway-requests
    #[clap(long)]
    clients_port: Option<u16>,

    /// The host that will be reported to the directory server
    #[clap(long)]
    // TODO: could this be changed to `Option<url::Url>`?
    announce_host: Option<String>,

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
}

impl From<Init> for OverrideConfig {
    fn from(init_config: Init) -> Self {
        OverrideConfig {
            host: Some(init_config.host),
            wallet_address: Some(init_config.wallet_address),
            mix_port: init_config.mix_port,
            clients_port: init_config.clients_port,
            datastore: init_config.datastore,
            announce_host: init_config.announce_host,
            nym_apis: init_config.nym_apis,
            mnemonic: init_config.mnemonic,

            enabled_statistics: init_config.enabled_statistics,
            statistics_service_url: init_config.statistics_service_url,

            nyxd_urls: init_config.nyxd_urls,
            only_coconut_credentials: init_config.only_coconut_credentials,
        }
    }
}

pub async fn execute(args: Init, output: OutputFormat) -> Result<(), Box<dyn Error + Send + Sync>> {
    eprintln!("Initialising gateway {}...", args.id);

    let already_init = if Config::default_config_file_path(&args.id).exists() {
        eprintln!(
            "Gateway \"{}\" was already initialised before! Config information will be \
            overwritten (but keys will be kept)!",
            args.id
        );
        true
    } else {
        false
    };

    let override_config_fields = OverrideConfig::from(args.clone());

    // Initialising the config structure is just overriding a default constructed one
    let config = override_config(Config::new(&args.id), override_config_fields)?;

    // if gateway was already initialised, don't generate new keys
    if !already_init {
        let mut rng = rand::rngs::OsRng;

        let identity_keys = identity::KeyPair::new(&mut rng);
        let sphinx_keys = encryption::KeyPair::new(&mut rng);
        let pathfinder = GatewayPathfinder::new_from_config(&config);
        nym_pemstore::store_keypair(
            &sphinx_keys,
            &nym_pemstore::KeyPairPath::new(
                pathfinder.private_encryption_key().to_owned(),
                pathfinder.public_encryption_key().to_owned(),
            ),
        )
        .expect("Failed to save sphinx keys");

        nym_pemstore::store_keypair(
            &identity_keys,
            &nym_pemstore::KeyPairPath::new(
                pathfinder.private_identity_key().to_owned(),
                pathfinder.public_identity_key().to_owned(),
            ),
        )
        .expect("Failed to save identity keys");

        eprintln!("Saved identity and mixnet sphinx keypairs");
    }

    let config_save_location = config.get_config_file_save_location();
    config
        .save_to_file(None)
        .expect("Failed to save the config file");
    eprintln!("Saved configuration file to {:?}", config_save_location);
    eprintln!("Gateway configuration completed.\n\n\n");

    Ok(crate::node::create_gateway(config)
        .await
        .print_node_details(output)?)
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
            wallet_address: "n1z9egw0knv47nmur0p8vk4rcx59h9gg4zjx9ede".parse().unwrap(),
            mix_port: Some(42),
            clients_port: Some(43),
            announce_host: Some("foo-announce-host".to_string()),
            datastore: Some("/foo-datastore".parse().unwrap()),
            nym_apis: None,
            mnemonic: None,
            statistics_service_url: None,
            enabled_statistics: None,
            nyxd_urls: None,
            only_coconut_credentials: None,
        };
        std::env::set_var(BECH32_PREFIX, "n");

        let config = Config::new(&args.id);
        let config = override_config(config, OverrideConfig::from(args.clone())).unwrap();

        let (identity_keys, sphinx_keys) = {
            let mut rng = rand::rngs::OsRng;
            (
                identity::KeyPair::new(&mut rng),
                encryption::KeyPair::new(&mut rng),
            )
        };

        // The test is really if this instantiates with InMemStorage without panics
        let _gateway =
            Gateway::new_from_keys_and_storage(config, identity_keys, sphinx_keys, InMemStorage)
                .await;
    }
}

// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::commands::helpers::{
    initialise_local_ip_packet_router, initialise_local_network_requester,
    OverrideNetworkRequesterConfig,
};
use crate::config::{default_config_directory, default_config_filepath, default_data_directory};
use crate::node::helpers::node_details;
use crate::{commands::helpers::OverrideConfig, config::Config, OutputFormat};
use anyhow::bail;
use clap::Args;
use nym_crypto::asymmetric::{encryption, identity};
use std::net::IpAddr;
use std::path::PathBuf;
use std::{fs, io};

use super::helpers::OverrideIpPacketRouterConfig;

#[derive(Args, Clone)]
pub struct Init {
    /// Id of the gateway we want to create config for
    #[clap(long)]
    id: String,

    /// The listening address on which the gateway will be receiving sphinx packets and listening for client data
    #[clap(long, alias = "host")]
    listening_address: IpAddr,

    /// Comma separated list of public ip addresses that will announced to the nym-api and subsequently to the clients.
    /// In nearly all circumstances, it's going to be identical to the address you're going to use for bonding.
    #[clap(long, value_delimiter = ',')]
    public_ips: Option<Vec<IpAddr>>,

    /// Optional hostname associated with this gateway that will announced to the nym-api and subsequently to the clients
    #[clap(long)]
    hostname: Option<String>,

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
    #[clap(long, conflicts_with = "with_ip_packet_router")]
    with_network_requester: bool,

    /// Allows this gateway to run an embedded network requester for minimal network overhead
    #[clap(long, hide = true, conflicts_with = "with_network_requester")]
    with_ip_packet_router: bool,

    // ##### NETWORK REQUESTER FLAGS #####
    /// Specifies whether this network requester should run in 'open-proxy' mode
    #[clap(long, requires = "with_network_requester")]
    open_proxy: Option<bool>,

    /// Enable service anonymized statistics that get sent to a statistics aggregator server
    #[clap(long, requires = "with_network_requester")]
    enable_statistics: Option<bool>,

    /// Mixnet client address where a statistics aggregator is running. The default value is a Nym
    /// aggregator client
    #[clap(long, requires = "with_network_requester")]
    statistics_recipient: Option<String>,

    /// Mostly debug-related option to increase default traffic rate so that you would not need to
    /// modify config post init
    #[clap(
        long,
        hide = true,
        conflicts_with = "medium_toggle",
        requires = "with_network_requester"
    )]
    fastmode: bool,

    /// Disable loop cover traffic and the Poisson rate limiter (for debugging only)
    #[clap(
        long,
        hide = true,
        conflicts_with = "medium_toggle",
        requires = "with_network_requester"
    )]
    no_cover: bool,

    /// Enable medium mixnet traffic, for experiments only.
    /// This includes things like disabling cover traffic, no per hop delays, etc.
    #[clap(
        long,
        hide = true,
        conflicts_with = "no_cover",
        conflicts_with = "fastmode",
        requires = "with_network_requester"
    )]
    medium_toggle: bool,

    /// Specifies whether this network requester will run using the default ExitPolicy
    /// as opposed to the allow list.
    /// Note: this setting will become the default in the future releases.
    #[clap(long)]
    with_exit_policy: Option<bool>,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

impl From<Init> for OverrideConfig {
    fn from(init_config: Init) -> Self {
        OverrideConfig {
            listening_address: Some(init_config.listening_address),
            public_ips: init_config.public_ips,
            hostname: init_config.hostname,
            mix_port: init_config.mix_port,
            clients_port: init_config.clients_port,
            datastore: init_config.datastore,
            nym_apis: init_config.nym_apis,
            mnemonic: init_config.mnemonic,

            enabled_statistics: init_config.enabled_statistics,
            statistics_service_url: init_config.statistics_service_url,

            nyxd_urls: init_config.nyxd_urls,
            only_coconut_credentials: init_config.only_coconut_credentials,
            with_network_requester: Some(init_config.with_network_requester),
            with_ip_packet_router: Some(init_config.with_ip_packet_router),
        }
    }
}

impl<'a> From<&'a Init> for OverrideNetworkRequesterConfig {
    fn from(value: &'a Init) -> Self {
        OverrideNetworkRequesterConfig {
            fastmode: value.fastmode,
            no_cover: value.no_cover,
            medium_toggle: value.medium_toggle,
            open_proxy: value.open_proxy,
            enable_exit_policy: value.with_exit_policy,
            enable_statistics: value.enable_statistics,
            statistics_recipient: value.statistics_recipient.clone(),
        }
    }
}

impl From<&Init> for OverrideIpPacketRouterConfig {
    fn from(_value: &Init) -> Self {
        OverrideIpPacketRouterConfig {}
    }
}

fn init_paths(id: &str) -> io::Result<()> {
    fs::create_dir_all(default_data_directory(id))?;
    fs::create_dir_all(default_config_directory(id))
}

pub async fn execute(args: Init) -> anyhow::Result<()> {
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
    let nr_opts = (&args).into();
    let ip_opts = (&args).into();
    let mut config = OverrideConfig::from(args).do_override(fresh_config)?;

    // if gateway was already initialised, don't generate new keys, et al.
    if !already_init {
        let mut rng = rand::rngs::OsRng;

        let identity_keys = identity::KeyPair::new(&mut rng);
        let sphinx_keys = encryption::KeyPair::new(&mut rng);

        if let Err(err) = nym_pemstore::store_keypair(
            &identity_keys,
            &nym_pemstore::KeyPairPath::new(
                config.storage_paths.private_identity_key(),
                config.storage_paths.public_identity_key(),
            ),
        ) {
            bail!("failed to save the identity keys: {err}")
        }

        if let Err(err) = nym_pemstore::store_keypair(
            &sphinx_keys,
            &nym_pemstore::KeyPairPath::new(
                config.storage_paths.private_encryption_key(),
                config.storage_paths.public_encryption_key(),
            ),
        ) {
            bail!("failed to save the sphinx keys: {err}")
        }

        if config.network_requester.enabled {
            initialise_local_network_requester(&config, nr_opts, *identity_keys.public_key())
                .await?;
        } else if config.ip_packet_router.enabled {
            initialise_local_ip_packet_router(&config, ip_opts, *identity_keys.public_key())
                .await?;
        }

        eprintln!("Saved identity and mixnet sphinx keypairs");
    }

    let config_save_location = config.default_location();
    if let Err(err) = config.save_to_default_location() {
        bail!("failed to save the config file: {err}")
    }
    config.save_path = Some(config_save_location.clone());

    eprintln!(
        "Saved configuration file to {}",
        config_save_location.display()
    );
    eprintln!("Gateway configuration completed.\n\n\n");

    output.to_stdout(&node_details(&config)?);

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
            listening_address: "1.1.1.1".parse().unwrap(),
            public_ips: None,
            hostname: None,
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
            with_network_requester: false,
            with_ip_packet_router: false,
            open_proxy: None,
            enable_statistics: None,
            statistics_recipient: None,
            fastmode: false,
            no_cover: false,
            medium_toggle: false,
            with_exit_policy: None,
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
            None,
            identity_keys,
            sphinx_keys,
            InMemStorage,
        )
        .await;
    }
}

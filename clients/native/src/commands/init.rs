// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::config::default_config_filepath;
use crate::commands::try_upgrade_v1_1_13_config;
use crate::{
    client::config::Config,
    commands::{override_config, OverrideConfig},
    error::ClientError,
};
use clap::Args;
use nym_bin_common::output_format::OutputFormat;
use nym_client_core::client::key_manager::persistence::OnDiskKeys;
use nym_crypto::asymmetric::identity;
use nym_sphinx::addressing::clients::Recipient;
use serde::Serialize;
use std::fmt::Display;
use std::net::IpAddr;
use tap::TapFallible;

#[derive(Args, Clone)]
pub(crate) struct Init {
    /// Id of the nym-mixnet-client we want to create config for.
    #[clap(long)]
    id: String,

    /// Id of the gateway we are going to connect to.
    #[clap(long)]
    gateway: Option<identity::PublicKey>,

    /// Specifies whether the new gateway should be determined based by latency as opposed to being chosen
    /// uniformly.
    #[clap(long, conflicts_with = "gateway")]
    latency_based_selection: bool,

    /// Force register gateway. WARNING: this will overwrite any existing keys for the given id,
    /// potentially causing loss of access.
    #[clap(long)]
    force_register_gateway: bool,

    /// Comma separated list of rest endpoints of the nyxd validators
    #[clap(long, alias = "nyxd_validators", value_delimiter = ',', hide = true)]
    nyxd_urls: Option<Vec<url::Url>>,

    /// Comma separated list of rest endpoints of the API validators
    #[clap(long, alias = "api_validators", value_delimiter = ',')]
    // the alias here is included for backwards compatibility (1.1.4 and before)
    nym_apis: Option<Vec<url::Url>>,

    /// Whether to not start the websocket
    #[clap(long)]
    disable_socket: Option<bool>,

    /// Port for the socket (if applicable) to listen on in all subsequent runs
    #[clap(short, long)]
    port: Option<u16>,

    /// Ip for the socket (if applicable) to listen for requests.
    #[clap(long)]
    host: Option<IpAddr>,

    /// Mostly debug-related option to increase default traffic rate so that you would not need to
    /// modify config post init
    #[clap(long, hide = true)]
    fastmode: bool,

    /// Disable loop cover traffic and the Poisson rate limiter (for debugging only)
    #[clap(long, hide = true)]
    no_cover: bool,

    /// Set this client to work in a enabled credentials mode that would attempt to use gateway
    /// with bandwidth credential requirement.
    #[clap(long, hide = true)]
    enabled_credentials_mode: Option<bool>,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

impl From<Init> for OverrideConfig {
    fn from(init_config: Init) -> Self {
        OverrideConfig {
            nym_apis: init_config.nym_apis,
            disable_socket: init_config.disable_socket,
            port: init_config.port,
            host: init_config.host,
            fastmode: init_config.fastmode,
            no_cover: init_config.no_cover,

            nyxd_urls: init_config.nyxd_urls,
            enabled_credentials_mode: init_config.enabled_credentials_mode,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct InitResults {
    #[serde(flatten)]
    client_core: nym_client_core::init::InitResults,
    client_listening_port: u16,
    client_address: String,
}

impl InitResults {
    fn new(config: &Config, address: &Recipient) -> Self {
        Self {
            client_core: nym_client_core::init::InitResults::new(&config.base, address),
            client_listening_port: config.socket.listening_port,
            client_address: address.to_string(),
        }
    }
}

impl Display for InitResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.client_core)?;
        writeln!(f, "Client listening port: {}", self.client_listening_port)?;
        write!(f, "Address of this client: {}", self.client_address)
    }
}

pub(crate) async fn execute(args: &Init) -> Result<(), ClientError> {
    eprintln!("Initialising client...");

    let id = &args.id;

    let already_init = if default_config_filepath(&id).exists() {
        // in case we're using old config, try to upgrade it
        // (if we're using the current version, it's a no-op)
        try_upgrade_v1_1_13_config(id)?;
        eprintln!("Client \"{id}\" was already initialised before");
        true
    } else {
        false
    };

    // Usually you only register with the gateway on the first init, however you can force
    // re-registering if wanted.
    let user_wants_force_register = args.force_register_gateway;
    if user_wants_force_register {
        eprintln!("Instructed to force registering gateway. This might overwrite keys!");
    }

    // If the client was already initialized, don't generate new keys and don't re-register with
    // the gateway (because this would create a new shared key).
    // Unless the user really wants to.
    let register_gateway = !already_init || user_wants_force_register;

    // Attempt to use a user-provided gateway, if possible
    let user_chosen_gateway_id = args.gateway;

    // Load and potentially override config
    let mut config = override_config(Config::new(id), OverrideConfig::from(args.clone()));

    // Setup gateway by either registering a new one, or creating a new config from the selected
    // one but with keys kept, or reusing the gateway configuration.
    let key_store = OnDiskKeys::new(config.paths.keys_pathfinder.clone());
    let gateway = nym_client_core::init::setup_gateway_from_config::<_>(
        &key_store,
        register_gateway,
        user_chosen_gateway_id,
        &config.base,
        args.latency_based_selection,
    )
    .await
    .tap_err(|err| eprintln!("Failed to setup gateway\nError: {err}"))?;

    config.base.set_gateway_endpoint(gateway);

    let config_save_location = config.default_location();
    config.save_to_default_location().tap_err(|_| {
        log::error!("Failed to save the config file");
    })?;
    eprintln!(
        "Saved configuration file to {}",
        config_save_location.display()
    );

    let address = nym_client_core::init::get_client_address_from_stored_ondisk_keys(
        &config.paths.keys_pathfinder,
        &config.base.client.gateway_endpoint,
    )?;

    eprintln!("Client configuration completed.\n");

    let init_results = InitResults::new(&config, &address);
    println!("{}", args.output.format(&init_results));

    Ok(())
}

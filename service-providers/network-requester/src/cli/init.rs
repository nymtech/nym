// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    cli::{override_config, OverrideConfig},
    config::Config,
    error::NetworkRequesterError,
};
use clap::Args;
use nym_config::NymConfig;
use nym_crypto::asymmetric::identity;
use nym_sphinx::addressing::clients::Recipient;
use serde::Serialize;
use std::fmt::Display;
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
    #[clap(long, alias = "nymd_validators", value_delimiter = ',')]
    nyxd_urls: Option<Vec<url::Url>>,

    /// Comma separated list of rest endpoints of the API validators
    #[clap(long, alias = "api_validators", value_delimiter = ',')]
    // the alias here is included for backwards compatibility (1.1.4 and before)
    nym_apis: Option<Vec<url::Url>>,

    /// Set this client to work in a enabled credentials mode that would attempt to use gateway
    /// with bandwidth credential requirement.
    #[clap(long)]
    enabled_credentials_mode: Option<bool>,

    /// Save a summary of the initialization to a json file
    #[clap(long)]
    output_json: bool,
}

impl From<Init> for OverrideConfig {
    fn from(init_config: Init) -> Self {
        OverrideConfig {
            nym_apis: init_config.nym_apis,
            fastmode: false,
            no_cover: false,

            nyxd_urls: init_config.nyxd_urls,
            enabled_credentials_mode: init_config.enabled_credentials_mode,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct InitResults {
    #[serde(flatten)]
    client_core: client_core::init::InitResults,
}

impl InitResults {
    fn new(config: &Config, address: &Recipient) -> Self {
        Self {
            client_core: client_core::init::InitResults::new(config.get_base(), address),
        }
    }
}

impl Display for InitResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.client_core)
    }
}

pub(crate) async fn execute(args: &Init) -> Result<(), NetworkRequesterError> {
    println!("Initialising client...");

    let id = &args.id;

    let already_init = Config::default_config_file_path(id).exists();
    if already_init {
        println!("Client \"{id}\" was already initialised before");
    }

    // Usually you only register with the gateway on the first init, however you can force
    // re-registering if wanted.
    let user_wants_force_register = args.force_register_gateway;
    if user_wants_force_register {
        println!("Instructed to force registering gateway. This might overwrite keys!");
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
    let gateway = client_core::init::setup_gateway_from_config::<Config, _>(
        register_gateway,
        user_chosen_gateway_id,
        config.get_base(),
        args.latency_based_selection,
    )
    .await
    .map_err(|source| {
        eprintln!("Failed to setup gateway\nError: {source}");
        NetworkRequesterError::FailedToSetupGateway { source }
    })?;

    config.get_base_mut().set_gateway_endpoint(gateway);

    config.save_to_file(None).tap_err(|_| {
        log::error!("Failed to save the config file");
    })?;

    print_saved_config(&config);

    let address = client_core::init::get_client_address_from_stored_keys(config.get_base())?;
    let init_results = InitResults::new(&config, &address);
    println!("{init_results}");

    // Output summary to a json file, if specified
    if args.output_json {
        client_core::init::output_to_json(&init_results, "client_init_results.json");
    }

    println!("\nThe address of this client is: {address}\n");
    Ok(())
}

fn print_saved_config(config: &Config) {
    let config_save_location = config.get_config_file_save_location();
    println!("Saved configuration file to {config_save_location:?}");
    println!("Using gateway: {}", config.get_base().get_gateway_id());
    log::debug!("Gateway id: {}", config.get_base().get_gateway_id());
    log::debug!("Gateway owner: {}", config.get_base().get_gateway_owner());
    log::debug!(
        "Gateway listener: {}",
        config.get_base().get_gateway_listener()
    );
    println!("Client configuration completed.\n");
}

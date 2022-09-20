// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Args;
use client_core::{config::GatewayEndpoint, error::ClientCoreError};
use config::NymConfig;

use crate::{
    client::config::Config,
    commands::{override_config, OverrideConfig},
};

#[derive(Args, Clone)]
pub(crate) struct Init {
    /// Id of the nym-mixnet-client we want to create config for.
    #[clap(long)]
    id: String,

    /// Id of the gateway we are going to connect to.
    #[clap(long)]
    gateway: Option<String>,

    /// Force register gateway. WARNING: this will overwrite any existing keys for the given id,
    /// potentially causing loss of access.
    #[clap(long)]
    force_register_gateway: bool,

    /// Comma separated list of rest endpoints of the validators
    #[clap(long)]
    validators: Option<String>,

    /// Whether to not start the websocket
    #[clap(long)]
    disable_socket: bool,

    /// Port for the socket (if applicable) to listen on in all subsequent runs
    #[clap(short, long)]
    port: Option<u16>,

    /// Mostly debug-related option to increase default traffic rate so that you would not need to
    /// modify config post init
    #[clap(long, hidden = true)]
    fastmode: bool,

    /// Set this client to work in a enabled credentials mode that would attempt to use gateway
    /// with bandwidth credential requirement.
    #[cfg(feature = "coconut")]
    #[clap(long)]
    enabled_credentials_mode: bool,
}

impl From<Init> for OverrideConfig {
    fn from(init_config: Init) -> Self {
        OverrideConfig {
            validators: init_config.validators,
            disable_socket: init_config.disable_socket,
            port: init_config.port,
            fastmode: init_config.fastmode,

            #[cfg(feature = "coconut")]
            enabled_credentials_mode: init_config.enabled_credentials_mode,
        }
    }
}

pub(crate) async fn execute(args: &Init) {
    println!("Initialising client...");

    let id = &args.id;

    let already_init = Config::default_config_file_path(Some(id)).exists();
    if already_init {
        println!(
            "Client \"{}\" was already initialised before! \
            Config information will be overwritten (but keys will be kept)!",
            id
        );
    }

    // Usually you only register with the gateway on the first init, however you can force
    // re-registering if wanted.
    let user_wants_force_register = args.force_register_gateway;

    // If the client was already initialized, don't generate new keys and don't re-register with
    // the gateway (because this would create a new shared key).
    // Unless the user really wants to.
    let register_gateway = !already_init || user_wants_force_register;

    // Attempt to use a user-provided gateway, if possible
    let user_chosen_gateway_id = args.gateway.as_deref();

    let mut config = Config::new(id);
    let override_config_fields = OverrideConfig::from(args.clone());
    config = override_config(config, override_config_fields);

    let gateway = setup_gateway(id, register_gateway, user_chosen_gateway_id, &config)
        .await
        .unwrap_or_else(|err| {
            eprintln!("Failed to setup gateway\nError: {err}");
            std::process::exit(1)
        });
    config.get_base_mut().with_gateway_endpoint(gateway);

    let config_save_location = config.get_config_file_save_location();
    config
        .save_to_file(None)
        .expect("Failed to save the config file");

    println!("Saved configuration file to {:?}", config_save_location);
    println!("Using gateway: {}", config.get_base().get_gateway_id());
    log::debug!("Gateway id: {}", config.get_base().get_gateway_id());
    log::debug!("Gateway owner: {}", config.get_base().get_gateway_owner());
    log::debug!(
        "Gateway listener: {}",
        config.get_base().get_gateway_listener()
    );
    println!("Client configuration completed.");

    client_core::init::show_address(config.get_base()).unwrap_or_else(|err| {
        eprintln!("Failed to show address\nError: {err}");
        std::process::exit(1)
    });
}

async fn setup_gateway(
    id: &str,
    register: bool,
    user_chosen_gateway_id: Option<&str>,
    config: &Config,
) -> Result<GatewayEndpoint, ClientCoreError> {
    if register {
        // Get the gateway details by querying the validator-api. Either pick one at random or use
        // the chosen one if it's among the available ones.
        println!("Configuring gateway");
        let gateway = client_core::init::query_gateway_details(
            config.get_base().get_validator_api_endpoints(),
            user_chosen_gateway_id,
        )
        .await?;
        log::debug!("Querying gateway gives: {}", gateway);

        // Registering with gateway by setting up and writing shared keys to disk
        log::trace!("Registering gateway");
        client_core::init::register_with_gateway_and_store_keys(gateway.clone(), config.get_base())
            .await?;
        println!("Saved all generated keys");

        Ok(gateway.into())
    } else if user_chosen_gateway_id.is_some() {
        // Just set the config, don't register or create any keys
        // This assumes that the user knows what they are doing, and that the existing keys are
        // valid for the gateway being used
        println!("Using gateway provided by user, keeping existing keys");
        let gateway = client_core::init::query_gateway_details(
            config.get_base().get_validator_api_endpoints(),
            user_chosen_gateway_id,
        )
        .await?;
        log::debug!("Querying gateway gives: {}", gateway);
        Ok(gateway.into())
    } else {
        println!("Not registering gateway, will reuse existing config and keys");
        let existing_config = Config::load_from_file(Some(id)).map_err(|err| {
            log::error!(
                "Unable to configure gateway: {err}. \n
                Seems like the client was already initialized but it was not possible to read \
                the existing configuration file. \n
                CAUTION: Consider backing up your gateway keys and try force gateway registration, or \
                removing the existing configuration and starting over."
            );
            ClientCoreError::CouldNotLoadExistingGatewayConfiguration(err)
        })?;

        Ok(existing_config.get_base().get_gateway_endpoint().clone())
    }
}

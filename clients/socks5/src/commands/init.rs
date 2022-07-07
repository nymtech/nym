// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Args;
use client_core::config::GatewayEndpoint;
use config::NymConfig;

use crate::{
    client::config::Config,
    commands::{override_config, OverrideConfig},
};

#[cfg(all(feature = "eth", not(feature = "coconut")))]
use crate::commands::{DEFAULT_ETH_ENDPOINT, DEFAULT_ETH_PRIVATE_KEY};

#[derive(Args, Clone)]
pub(crate) struct Init {
    /// Id of the nym-mixnet-client we want to create config for.
    #[clap(long)]
    id: String,

    /// Address of the socks5 provider to send messages to.
    #[clap(long)]
    provider: String,

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

    /// Port for the socket to listen on in all subsequent runs")
    #[clap(short, long)]
    port: Option<u16>,

    /// Mostly debug-related option to increase default traffic rate so that you would not need to
    /// modify config post init
    #[clap(long, hidden = true)]
    fastmode: bool,

    /// Set this client to work in a enabled credentials mode that would attempt to use gateway
    /// with bandwidth credential requirement. If this value is set, --eth-endpoint and
    /// --eth-private_key don't need to be set.
    #[cfg(all(feature = "eth", not(feature = "coconut")))]
    #[clap(long, conflicts_with_all = &["eth-endpoint", "eth-private-key"])]
    enabled_credentials_mode: bool,

    /// URL of an Ethereum full node that we want to use for getting bandwidth tokens from ERC20
    /// tokens. If you don't want to set this value, use --enabled-credentials-mode instead
    #[cfg(all(feature = "eth", not(feature = "coconut")))]
    #[clap(
        long,
        default_value = DEFAULT_ETH_ENDPOINT
    )]
    eth_endpoint: String,

    /// Ethereum private key used for obtaining bandwidth tokens from ERC20 tokens. If you don't
    /// want to set this value, use --enabled-credentials-mode instead")
    #[cfg(all(feature = "eth", not(feature = "coconut")))]
    #[clap(
        long,
        default_value = DEFAULT_ETH_PRIVATE_KEY
    )]
    eth_private_key: String,
}

impl From<Init> for OverrideConfig {
    fn from(init_config: Init) -> Self {
        OverrideConfig {
            validators: init_config.validators,
            port: init_config.port,
            fastmode: init_config.fastmode,

            #[cfg(all(feature = "eth", not(feature = "coconut")))]
            enabled_credentials_mode: init_config.enabled_credentials_mode,

            #[cfg(all(feature = "eth", not(feature = "coconut")))]
            eth_private_key: Some(init_config.eth_private_key),

            #[cfg(all(feature = "eth", not(feature = "coconut")))]
            eth_endpoint: Some(init_config.eth_endpoint),
        }
    }
}

pub(crate) async fn execute(args: &Init) {
    println!("Initialising client...");

    let id = &args.id;
    let provider_address = &args.provider;

    let already_init = Config::default_config_file_path(Some(id)).exists();
    if already_init {
        println!(
            "SOCKS5 client \"{}\" was already initialised before! \
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

    let mut config = Config::new(id, provider_address);
    let override_config_fields = OverrideConfig::from(args.clone());
    config = override_config(config, override_config_fields);

    let gateway = setup_gateway(id, register_gateway, user_chosen_gateway_id, &config).await;
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

    client_core::init::show_address(config.get_base());
}

async fn setup_gateway(
    id: &str,
    register: bool,
    user_chosen_gateway_id: Option<&str>,
    config: &Config,
) -> GatewayEndpoint {
    if register {
        // Get the gateway details by querying the validator-api. Either pick one at random or use
        // the chosen one if it's among the available ones.
        println!("Configuring gateway");
        let gateway = client_core::init::query_gateway_details(
            config.get_base().get_validator_api_endpoints(),
            user_chosen_gateway_id,
        )
        .await;
        log::debug!("Querying gateway gives: {}", gateway);

        // Registering with gateway by setting up and writing shared keys to disk
        log::trace!("Registering gateway");
        client_core::init::register_with_gateway_and_store_keys(gateway.clone(), config.get_base())
            .await;
        println!("Saved all generated keys");

        gateway.into()
    } else if user_chosen_gateway_id.is_some() {
        // Just set the config, don't register or create any keys
        // This assumes that the user knows what they are doing, and that the existing keys are
        // valid for the gateway being used
        println!("Using gateway provided by user, keeping existing keys");
        let gateway = client_core::init::query_gateway_details(
            config.get_base().get_validator_api_endpoints(),
            user_chosen_gateway_id,
        )
        .await;
        log::debug!("Querying gateway gives: {}", gateway);
        gateway.into()
    } else {
        println!("Not registering gateway, will reuse existing config and keys");
        match Config::load_from_file(Some(id)) {
            Ok(existing_config) => existing_config.get_base().get_gateway_endpoint().clone(),
            Err(err) => {
                panic!(
                    "Unable to configure gateway: {err}. \n
                    Seems like the client was already initialized but it was not possible to read \
                    the existing configuration file. \n
                    CAUTION: Consider backing up your gateway keys and try force gateway registration, or \
                    removing the existing configuration and starting over."
                )
            }
        }
    }
}

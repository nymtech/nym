// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{App, Arg, ArgMatches};
use client_core::config::GatewayEndpoint;
use config::NymConfig;

use crate::client::config::Config;
use crate::commands::override_config;
#[cfg(feature = "eth")]
#[cfg(not(feature = "coconut"))]
use crate::commands::{
    DEFAULT_ETH_ENDPOINT, DEFAULT_ETH_PRIVATE_KEY, ENABLED_CREDENTIALS_MODE_ARG_NAME,
    ETH_ENDPOINT_ARG_NAME, ETH_PRIVATE_KEY_ARG_NAME,
};

pub fn command_args<'a, 'b>() -> clap::App<'a, 'b> {
    let app = App::new("init")
        .about("Initialise a Nym client. Do this first!")
        .arg(Arg::with_name("id")
            .long("id")
            .help("Id of the nym-mixnet-client we want to create config for.")
            .takes_value(true)
            .required(true)
        )
        .arg(Arg::with_name("gateway")
            .long("gateway")
            .help("Id of the gateway we are going to connect to.")
            .takes_value(true)
        )
        .arg(Arg::with_name("force-register-gateway")
            .long("force-register-gateway")
            .help("Force register gateway. WARNING: this will overwrite any existing keys for the given id, potentially causing loss of access.")
            .takes_value(false)
        )
        .arg(Arg::with_name("validators")
                 .long("validators")
                 .help("Comma separated list of rest endpoints of the validators")
                 .takes_value(true),
        )
        .arg(Arg::with_name("disable-socket")
            .long("disable-socket")
            .help("Whether to not start the websocket")
        )
        .arg(Arg::with_name("port")
            .short("p")
            .long("port")
            .help("Port for the socket (if applicable) to listen on in all subsequent runs")
            .takes_value(true)
        )
        .arg(Arg::with_name("fastmode")
            .long("fastmode")
            .hidden(true) // this will prevent this flag from being displayed in `--help`
            .help("Mostly debug-related option to increase default traffic rate so that you would not need to modify config post init")
        );
    #[cfg(feature = "eth")]
    #[cfg(not(feature = "coconut"))]
        let app = app
        .arg(
            Arg::with_name(ENABLED_CREDENTIALS_MODE_ARG_NAME)
                .long(ENABLED_CREDENTIALS_MODE_ARG_NAME)
                .help("Set this client to work in a disabled credentials mode that would attempt to use gateway without bandwidth credential requirement. If this value is set, --eth_endpoint and --eth_private_key don't need to be set.")
                .conflicts_with_all(&[ETH_ENDPOINT_ARG_NAME, ETH_PRIVATE_KEY_ARG_NAME])
        )
        .arg(Arg::with_name(ETH_ENDPOINT_ARG_NAME)
            .long(ETH_ENDPOINT_ARG_NAME)
            .help("URL of an Ethereum full node that we want to use for getting bandwidth tokens from ERC20 tokens. If you don't want to set this value, use --testnet-mode instead")
            .takes_value(true)
            .default_value_if(ENABLED_CREDENTIALS_MODE_ARG_NAME, None, DEFAULT_ETH_ENDPOINT)
            .required(true))
        .arg(Arg::with_name(ETH_PRIVATE_KEY_ARG_NAME)
            .long(ETH_PRIVATE_KEY_ARG_NAME)
            .help("Ethereum private key used for obtaining bandwidth tokens from ERC20 tokens. If you don't want to set this value, use --testnet-mode instead")
            .takes_value(true)
            .default_value_if(ENABLED_CREDENTIALS_MODE_ARG_NAME, None, DEFAULT_ETH_PRIVATE_KEY)
            .required(true)
        );

    app
}

pub async fn execute(matches: ArgMatches<'static>) {
    println!("Initialising client...");

    let id = matches.value_of("id").unwrap(); // required for now

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
    let user_wants_force_register = matches.is_present("force-register-gateway");

    // If the client was already initialized, don't generate new keys and don't re-register with
    // the gateway (because this would create a new shared key).
    // Unless the user really wants to.
    let register_gateway = !already_init || user_wants_force_register;

    // Attempt to use a user-provided gateway, if possible
    let user_chosen_gateway_id = matches.value_of("gateway");

    let mut config = Config::new(id);

    config = override_config(config, &matches);
    if matches.is_present("fastmode") {
        config.get_base_mut().set_high_default_traffic_volume();
    }

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

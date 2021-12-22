// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::config::Config;
use crate::client::NymClient;
use crate::commands::override_config;
#[cfg(feature = "eth")]
use crate::commands::{ETH_ENDPOINT_ARG_NAME, ETH_PRIVATE_KEY_ARG_NAME, TESTNET_MODE_ARG_NAME};
use clap::{App, Arg, ArgMatches};
use config::NymConfig;
use log::*;
use version_checker::is_minor_version_compatible;

pub fn command_args<'a, 'b>() -> clap::App<'a, 'b> {
    let app = App::new("run")
        .about("Run the Nym client with provided configuration client optionally overriding set parameters")
        .arg(Arg::with_name("id")
            .long("id")
            .help("Id of the nym-mixnet-client we want to run.")
            .takes_value(true)
            .required(true)
        )
        // the rest of arguments are optional, they are used to override settings in config file
        .arg(Arg::with_name("validators")
                .long("validators")
                .help("Comma separated list rest rest endpoints of the validators")
                .takes_value(true),
        )
        .arg(Arg::with_name("gateway")
            .long("gateway")
            .help("Id of the gateway we want to connect to. If overridden, it is user's responsibility to ensure prior registration happened")
            .takes_value(true)
        )
        .arg(Arg::with_name("disable-socket")
            .long("disable-socket")
            .help("Whether to not start the websocket")
        )
        .arg(Arg::with_name("port")
            .short("p")
            .long("port")
            .help("Port for the socket (if applicable) to listen on")
            .takes_value(true)
        );
    #[cfg(feature = "eth")]
        let app = app
        .arg(
            Arg::with_name(TESTNET_MODE_ARG_NAME)
                .long(TESTNET_MODE_ARG_NAME)
                .help("Set this client to work in a testnet mode that would attempt to use gateway without bandwidth credential requirement. If this value is set, --eth_endpoint and --eth_private_key don't need to be set.")
                .conflicts_with_all(&[ETH_ENDPOINT_ARG_NAME, ETH_PRIVATE_KEY_ARG_NAME])
        )
        .arg(Arg::with_name(ETH_ENDPOINT_ARG_NAME)
            .long(ETH_ENDPOINT_ARG_NAME)
            .help("URL of an Ethereum full node that we want to use for getting bandwidth tokens from ERC20 tokens. If you don't want to set this value, use --testnet-mode instead")
            .takes_value(true))
        .arg(Arg::with_name(ETH_PRIVATE_KEY_ARG_NAME)
            .long(ETH_PRIVATE_KEY_ARG_NAME)
            .help("Ethereum private key used for obtaining bandwidth tokens from ERC20 tokens. If you don't want to set this value, use --testnet-mode instead")
            .takes_value(true));

    app
}

// this only checks compatibility between config the binary. It does not take into consideration
// network version. It might do so in the future.
fn version_check(cfg: &Config) -> bool {
    let binary_version = env!("CARGO_PKG_VERSION");
    let config_version = cfg.get_base().get_version();
    if binary_version != config_version {
        warn!("The mixnode binary has different version than what is specified in config file! {} and {}", binary_version, config_version);
        if is_minor_version_compatible(binary_version, config_version) {
            info!("but they are still semver compatible. However, consider running the `upgrade` command");
            true
        } else {
            error!("and they are semver incompatible! - please run the `upgrade` command before attempting `run` again");
            false
        }
    } else {
        true
    }
}

pub fn execute(matches: &ArgMatches) {
    let id = matches.value_of("id").unwrap();

    let mut config = match Config::load_from_file(Some(id)) {
        Ok(cfg) => cfg,
        Err(err) => {
            error!("Failed to load config for {}. Are you sure you have run `init` before? (Error was: {})", id, err);
            return;
        }
    };

    config = override_config(config, matches);

    if !version_check(&config) {
        error!("failed the local version check");
        return;
    }

    NymClient::new(config).run_forever();
}

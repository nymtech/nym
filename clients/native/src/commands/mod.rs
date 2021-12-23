// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::config::{Config, SocketType};
use clap::ArgMatches;
use url::Url;

pub(crate) const TESTNET_MODE_ARG_NAME: &str = "testnet-mode";
#[cfg(not(feature = "coconut"))]
pub(crate) const ETH_ENDPOINT_ARG_NAME: &str = "eth_endpoint";
#[cfg(not(feature = "coconut"))]
pub(crate) const ETH_PRIVATE_KEY_ARG_NAME: &str = "eth_private_key";
#[cfg(not(feature = "coconut"))]
pub(crate) const DEFAULT_ETH_ENDPOINT: &str =
    "https://rinkeby.infura.io/v3/00000000000000000000000000000000";
#[cfg(not(feature = "coconut"))]
pub(crate) const DEFAULT_ETH_PRIVATE_KEY: &str =
    "0000000000000000000000000000000000000000000000000000000000000001";

pub(crate) mod init;
pub(crate) mod run;
pub(crate) mod upgrade;

fn parse_validators(raw: &str) -> Vec<Url> {
    raw.split(',')
        .map(|raw_validator| {
            raw_validator
                .trim()
                .parse()
                .expect("one of the provided validator api urls is invalid")
        })
        .collect()
}

pub(crate) fn override_config(mut config: Config, matches: &ArgMatches) -> Config {
    if let Some(raw_validators) = matches.value_of("validators") {
        config
            .get_base_mut()
            .set_custom_validator_apis(parse_validators(raw_validators));
    }

    if let Some(gateway_id) = matches.value_of("gateway") {
        config.get_base_mut().with_gateway_id(gateway_id);
    }

    if matches.is_present("disable-socket") {
        config = config.with_socket(SocketType::None);
    }

    if let Some(port) = matches.value_of("port").map(|port| port.parse::<u16>()) {
        if let Err(err) = port {
            // if port was overridden, it must be parsable
            panic!("Invalid port value provided - {:?}", err);
        }
        config = config.with_port(port.unwrap());
    }

    #[cfg(not(feature = "coconut"))]
    if let Some(eth_endpoint) = matches.value_of(ETH_ENDPOINT_ARG_NAME) {
        config.get_base_mut().with_eth_endpoint(eth_endpoint);
    } else {
        config
            .get_base_mut()
            .with_eth_endpoint(DEFAULT_ETH_ENDPOINT);
    }
    #[cfg(not(feature = "coconut"))]
    if let Some(eth_private_key) = matches.value_of(ETH_PRIVATE_KEY_ARG_NAME) {
        config.get_base_mut().with_eth_private_key(eth_private_key);
    } else {
        config
            .get_base_mut()
            .with_eth_private_key(DEFAULT_ETH_PRIVATE_KEY);
    }

    if !cfg!(feature = "eth") || matches.is_present(TESTNET_MODE_ARG_NAME) {
        config.get_base_mut().with_testnet_mode(true)
    }

    config
}

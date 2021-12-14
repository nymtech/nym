// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use clap::ArgMatches;
use url::Url;

pub(crate) mod init;
pub(crate) mod run;
pub(crate) mod sign;
pub(crate) mod upgrade;

pub(crate) const ID_ARG_NAME: &str = "id";
pub(crate) const HOST_ARG_NAME: &str = "host";
pub(crate) const MIX_PORT_ARG_NAME: &str = "mix-port";
pub(crate) const CLIENTS_PORT_ARG_NAME: &str = "clients-port";
pub(crate) const VALIDATOR_APIS_ARG_NAME: &str = "validator-apis";
#[cfg(not(feature = "coconut"))]
pub(crate) const VALIDATORS_ARG_NAME: &str = "validators";
#[cfg(not(feature = "coconut"))]
pub(crate) const COSMOS_MNEMONIC: &str = "mnemonic";
#[cfg(not(feature = "coconut"))]
pub(crate) const ETH_ENDPOINT: &str = "eth_endpoint";
pub(crate) const ANNOUNCE_HOST_ARG_NAME: &str = "announce-host";
pub(crate) const DATASTORE_PATH: &str = "datastore";
pub(crate) const TESTNET_MODE_ARG_NAME: &str = "testnet-mode";

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
    let mut was_host_overridden = false;
    if let Some(host) = matches.value_of(HOST_ARG_NAME) {
        config = config.with_listening_address(host);
        was_host_overridden = true;
    }

    if let Some(mix_port) = matches
        .value_of(MIX_PORT_ARG_NAME)
        .map(|port| port.parse::<u16>())
    {
        if let Err(err) = mix_port {
            // if port was overridden, it must be parsable
            panic!("Invalid port value provided - {:?}", err);
        }
        config = config.with_mix_port(mix_port.unwrap());
    }

    if let Some(clients_port) = matches
        .value_of(CLIENTS_PORT_ARG_NAME)
        .map(|port| port.parse::<u16>())
    {
        if let Err(err) = clients_port {
            // if port was overridden, it must be parsable
            panic!("Invalid port value provided - {:?}", err);
        }
        config = config.with_clients_port(clients_port.unwrap());
    }

    if let Some(announce_host) = matches.value_of(ANNOUNCE_HOST_ARG_NAME) {
        config = config.with_announce_address(announce_host);
    } else if was_host_overridden {
        // make sure our 'mix-announce-host' always defaults to 'mix-host'
        config = config.announce_host_from_listening_host()
    }

    if let Some(raw_validators) = matches.value_of(VALIDATOR_APIS_ARG_NAME) {
        config = config.with_custom_validator_apis(parse_validators(raw_validators));
    }

    #[cfg(not(feature = "coconut"))]
    if let Some(raw_validators) = matches.value_of(VALIDATORS_ARG_NAME) {
        config = config.with_custom_validator_nymd(parse_validators(raw_validators));
    }

    #[cfg(not(feature = "coconut"))]
    if let Some(cosmos_mnemonic) = matches.value_of(COSMOS_MNEMONIC) {
        config = config.with_cosmos_mnemonic(String::from(cosmos_mnemonic));
    }

    if let Some(datastore_path) = matches.value_of(DATASTORE_PATH) {
        config = config.with_custom_persistent_store(datastore_path);
    }

    #[cfg(not(feature = "coconut"))]
    if let Some(eth_endpoint) = matches.value_of(ETH_ENDPOINT) {
        config = config.with_eth_endpoint(String::from(eth_endpoint));
    }

    config.with_testnet_mode(matches.is_present(TESTNET_MODE_ARG_NAME))
}

// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::config::Config;
use clap::ArgMatches;

pub(crate) mod init;
pub(crate) mod run;
pub(crate) mod upgrade;

fn parse_validators(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|raw_validator| raw_validator.trim().into())
        .collect()
}

pub(crate) fn override_config(mut config: Config, matches: &ArgMatches) -> Config {
    if let Some(raw_validators) = matches.value_of("validators") {
        config
            .get_base_mut()
            .set_custom_validators(parse_validators(raw_validators));
    }

    if let Some(contract_address) = matches.value_of("mixnet-contract") {
        config.get_base_mut().set_mixnet_contract(contract_address)
    }

    if let Some(gateway_id) = matches.value_of("gateway") {
        config.get_base_mut().with_gateway_id(gateway_id);
    }

    if let Some(port) = matches.value_of("port").map(|port| port.parse::<u16>()) {
        if let Err(err) = port {
            // if port was overridden, it must be parsable
            panic!("Invalid port value provided - {:?}", err);
        }
        config = config.with_port(port.unwrap());
    }

    config
}

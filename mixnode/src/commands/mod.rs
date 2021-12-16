// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use clap::ArgMatches;
use url::Url;

pub(crate) mod describe;
pub(crate) mod init;
pub(crate) mod run;
pub(crate) mod sign;
pub(crate) mod upgrade;

pub(crate) const ID_ARG_NAME: &str = "id";
pub(crate) const HOST_ARG_NAME: &str = "host";
pub(crate) const MIX_PORT_ARG_NAME: &str = "mix-port";
pub(crate) const VERLOC_PORT_ARG_NAME: &str = "verloc-port";
pub(crate) const HTTP_API_PORT_ARG_NAME: &str = "http-api-port";
pub(crate) const VALIDATORS_ARG_NAME: &str = "validators";
pub(crate) const ANNOUNCE_HOST_ARG_NAME: &str = "announce-host";

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

    if let Some(port) = matches
        .value_of(MIX_PORT_ARG_NAME)
        .map(|port| port.parse::<u16>())
    {
        if let Err(err) = port {
            // if port was overridden, it must be parsable
            panic!("Invalid mix port value provided - {:?}", err);
        }
        config = config.with_mix_port(port.unwrap());
    }

    if let Some(port) = matches
        .value_of(VERLOC_PORT_ARG_NAME)
        .map(|port| port.parse::<u16>())
    {
        if let Err(err) = port {
            // if port was overridden, it must be parsable
            panic!("Invalid verloc port value provided - {:?}", err);
        }
        config = config.with_verloc_port(port.unwrap());
    }

    if let Some(port) = matches
        .value_of(HTTP_API_PORT_ARG_NAME)
        .map(|port| port.parse::<u16>())
    {
        if let Err(err) = port {
            // if port was overridden, it must be parsable
            panic!("Invalid http api port value provided - {:?}", err);
        }
        config = config.with_http_api_port(port.unwrap());
    }

    if let Some(raw_validators) = matches.value_of(VALIDATORS_ARG_NAME) {
        config = config.with_custom_validator_apis(parse_validators(raw_validators));
    }

    if let Some(announce_host) = matches.value_of(ANNOUNCE_HOST_ARG_NAME) {
        config = config.with_announce_address(announce_host);
    } else if was_host_overridden {
        // make sure our 'announce-host' always defaults to 'host'
        config = config.announce_address_from_listening_address()
    }

    config
}

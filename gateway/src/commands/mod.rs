// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use clap::ArgMatches;
use url::Url;

pub(crate) mod init;
pub(crate) mod run;
pub(crate) mod upgrade;

pub(crate) const ID_ARG_NAME: &str = "id";
pub(crate) const HOST_ARG_NAME: &str = "host";
pub(crate) const MIX_PORT_ARG_NAME: &str = "mix-port";
pub(crate) const CLIENTS_PORT_ARG_NAME: &str = "clients-port";
pub(crate) const VALIDATORS_ARG_NAME: &str = "validators";
pub(crate) const ANNOUNCE_HOST_ARG_NAME: &str = "announce-host";
pub(crate) const INBOXES_ARG_NAME: &str = "inboxes";
pub(crate) const CLIENTS_LEDGER_ARG_NAME: &str = "clients-ledger";

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

    if let Some(raw_validators) = matches.value_of(VALIDATORS_ARG_NAME) {
        config = config.with_custom_validator_apis(parse_validators(raw_validators));
    }

    if let Some(inboxes_dir) = matches.value_of(INBOXES_ARG_NAME) {
        config = config.with_custom_clients_inboxes(inboxes_dir);
    }

    if let Some(clients_ledger) = matches.value_of(CLIENTS_LEDGER_ARG_NAME) {
        config = config.with_custom_clients_ledger(clients_ledger);
    }

    config
}

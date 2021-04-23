// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::client::config::{Config, SocketType};
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

    if matches.is_present("disable-socket") {
        config = config.with_socket(SocketType::None);
    }

    if matches.is_present("vpn-mode") {
        config.get_base_mut().set_vpn_mode(true);
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

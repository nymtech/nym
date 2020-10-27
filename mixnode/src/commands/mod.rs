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

use crate::config::Config;
use clap::ArgMatches;
use nymsphinx::params::DEFAULT_NUM_MIX_HOPS;

pub(crate) mod init;
pub(crate) mod run;
pub(crate) mod upgrade;

pub(crate) fn override_config(mut config: Config, matches: &ArgMatches) -> Config {
    let max_layer = DEFAULT_NUM_MIX_HOPS;
    let mut was_host_overridden = false;
    if let Some(host) = matches.value_of("host") {
        config = config.with_listening_host(host);
        was_host_overridden = true;
    }

    if let Some(layer) = matches.value_of("layer").map(|layer| layer.parse::<u64>()) {
        if let Err(err) = layer {
            // if layer was overridden, it must be parsable
            panic!("Invalid layer value provided - {:?}", err);
        }
        let layer = layer.unwrap();
        if layer <= max_layer as u64 && layer > 0 {
            config = config.with_layer(layer)
        }
    }

    if let Some(port) = matches.value_of("port").map(|port| port.parse::<u16>()) {
        if let Err(err) = port {
            // if port was overridden, it must be parsable
            panic!("Invalid port value provided - {:?}", err);
        }
        config = config.with_listening_port(port.unwrap());
    }

    if let Some(directory) = matches.value_of("validator") {
        config = config.with_custom_validator(directory);
    }

    if let Some(metrics_server) = matches.value_of("metrics-server") {
        config = config.with_custom_metrics_server(metrics_server);
    }

    if let Some(announce_host) = matches.value_of("announce-host") {
        config = config.with_announce_host(announce_host);
    } else if was_host_overridden {
        // make sure our 'announce-host' always defaults to 'host'
        config = config.announce_host_from_listening_host()
    }

    if let Some(announce_port) = matches
        .value_of("announce-port")
        .map(|port| port.parse::<u16>())
    {
        if let Err(err) = announce_port {
            // if port was overridden, it must be parsable
            panic!("Invalid port value provided - {:?}", err);
        }
        config = config.with_announce_port(announce_port.unwrap());
    }

    if let Some(location) = matches.value_of("location") {
        config = config.with_location(location);
    }

    config
}

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

use crate::client::config::Config;
use crate::client::NymClient;
use crate::commands::override_config;
use clap::{App, Arg, ArgMatches};
use config::NymConfig;
use log::*;

pub fn command_args<'a, 'b>() -> clap::App<'a, 'b> {
    App::new("run")
        .about("Run the Nym client with provided configuration client optionally overriding set parameters")
        .arg(Arg::with_name("id")
            .long("id")
            .help("Id of the nym-mixnet-client we want to run.")
            .takes_value(true)
            .required(true)
        )
        // the rest of arguments are optional, they are used to override settings in config file
        .arg(Arg::with_name("config")
            .long("config")
            .help("Custom path to the nym-mixnet-client configuration file")
            .takes_value(true)
        )
        .arg(Arg::with_name("provider")
            .long("provider")
            .help("Address of the socks5 provider to send messages to.")
            .takes_value(true)
        )
        .arg(Arg::with_name("directory")
            .long("directory")
            .help("Address of the directory server the client is getting topology from")
            .takes_value(true),
        )
        .arg(Arg::with_name("gateway")
            .long("gateway")
            .help("Id of the gateway we want to connect to. If overridden, it is user's responsibility to ensure prior registration happened")
            .takes_value(true)
        )
        .arg(Arg::with_name("vpn-mode")
            .long("vpn-mode")
            .help("Set the vpn mode of the client")
            .long_help(
                r#" 
                    Special mode of the system such that all messages are sent as soon as they are received
                    and no cover traffic is generated. If set all message delays are set to 0 and overwriting
                    'Debug' values will have no effect.
                "#
            )
        )
        .arg(Arg::with_name("port")
            .short("p")
            .long("port")
            .help("Port for the socket to listen on")
            .takes_value(true)
        )
}

pub fn execute(matches: &ArgMatches) {
    let id = matches.value_of("id").unwrap();

    let mut config = match Config::load_from_file(
        matches.value_of("config").map(|path| path.into()),
        Some(id),
    ) {
        Ok(cfg) => cfg,
        Err(err) => {
            error!("Failed to load config for {}. Are you sure you have run `init` before? (Error was: {})", id, err);
            return;
        }
    };

    config = override_config(config, matches);

    NymClient::new(config).run_forever();
}

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

use crate::client::NymClient;
use crate::commands::override_config;
use crate::config::{persistence::pathfinder::ClientPathfinder, Config};
use clap::{App, Arg, ArgMatches};
use config::NymConfig;
use crypto::asymmetric::identity;
use pemstore::pemstore::PemStore;

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
        .arg(Arg::with_name("disable-socket")
            .long("disable-socket")
            .help("Whether to not start the websocket")
        )
        .arg(Arg::with_name("port")
            .short("p")
            .long("port")
            .help("Port for the socket (if applicable) to listen on")
            .takes_value(true)
        )
}

fn load_identity_keys(config_file: &Config) -> identity::KeyPair {
    let identity_keypair = PemStore::new(ClientPathfinder::new_from_config(&config_file))
        .read_identity()
        .expect("Failed to read stored identity key files");
    println!(
        "Public identity key: {}\n",
        identity_keypair.public_key().to_base58_string()
    );
    identity_keypair
}

pub fn execute(matches: &ArgMatches) {
    let id = matches.value_of("id").unwrap();

    let mut config =
        Config::load_from_file(matches.value_of("config").map(|path| path.into()), Some(id))
            .expect("Failed to load config file");

    config = override_config(config, matches);
    let identity_keypair = load_identity_keys(&config);
    NymClient::new(config, identity_keypair).run_forever();
}

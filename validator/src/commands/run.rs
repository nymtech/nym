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

use crate::commands::override_config;
use crate::config::Config;
use crate::validator::Validator;
use clap::{App, Arg, ArgMatches};
use config::NymConfig;

pub fn command_args<'a, 'b>() -> clap::App<'a, 'b> {
    App::new("run")
        .about("Starts the validator")
        .arg(
            Arg::with_name("id")
                .long("id")
                .help("Id of the nym-validator we want to run")
                .takes_value(true)
                .required(true),
        )
        // the rest of arguments are optional, they are used to override settings in config file
        .arg(
            Arg::with_name("location")
                .long("location")
                .help("Optional geographical location of this node")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("config")
                .long("config")
                .help("Custom path to the nym-validator configuration file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("directory")
                .long("directory")
                .help("Address of the directory server the validator is sending presence to and uses for mix mining")
                .takes_value(true),
        )
}

pub fn execute(matches: &ArgMatches) {
    let id = matches.value_of("id").unwrap();

    println!("Starting validator {}...", id);

    let mut config =
        Config::load_from_file(matches.value_of("config").map(|path| path.into()), Some(id))
            .expect("Failed to load config file");

    config = override_config(config, matches);

    let validator = Validator::new(config);
    validator.start()
}

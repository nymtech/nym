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

use clap::{App, ArgMatches};

pub mod client;
mod commands;
pub mod socks;

fn main() {
    dotenv::dotenv().ok();
    setup_logging();
    println!("{}", banner());

    let arg_matches = App::new("Nym Socks5 Proxy")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Nymtech")
        .about("A Socks5 localhost proxy that converts incoming messages to Sphinx and sends them to a Nym address")
        .subcommand(commands::init::command_args())
        .subcommand(commands::run::command_args())
        .subcommand(commands::upgrade::command_args())
        .get_matches();

    execute(arg_matches);
}

fn execute(matches: ArgMatches) {
    match matches.subcommand() {
        ("init", Some(m)) => commands::init::execute(m),
        ("run", Some(m)) => commands::run::execute(m),
        ("upgrade", Some(m)) => commands::upgrade::execute(m),
        _ => println!("{}", usage()),
    }
}

fn usage() -> &'static str {
    "usage: --help to see available options.\n\n"
}

fn banner() -> String {
    format!(
        r#"

      _ __  _   _ _ __ ___
     | '_ \| | | | '_ \ _ \
     | | | | |_| | | | | | |
     |_| |_|\__, |_| |_| |_|
            |___/

             (socks5 proxy - version {:})

    "#,
        env!("CARGO_PKG_VERSION")
    )
}

fn setup_logging() {
    let mut log_builder = pretty_env_logger::formatted_timed_builder();
    if let Ok(s) = ::std::env::var("RUST_LOG") {
        log_builder.parse_filters(&s);
    } else {
        // default to 'Info'
        log_builder.filter(None, log::LevelFilter::Info);
    }

    log_builder
        .filter_module("hyper", log::LevelFilter::Warn)
        .filter_module("tokio_reactor", log::LevelFilter::Warn)
        .filter_module("reqwest", log::LevelFilter::Warn)
        .filter_module("mio", log::LevelFilter::Warn)
        .filter_module("want", log::LevelFilter::Warn)
        .filter_module("tungstenite", log::LevelFilter::Warn)
        .filter_module("tokio_tungstenite", log::LevelFilter::Warn)
        .init();
}

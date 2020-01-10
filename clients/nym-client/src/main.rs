#![recursion_limit = "256"]

use clap::{App, Arg, ArgMatches, SubCommand};
use env_logger;
use log::*;
use std::process;

pub mod clients;
mod commands;
mod persistence;
mod sockets;
mod utils;

fn main() {
    env_logger::init();

    let arg_matches = App::new("Nym Client")
        .version(built_info::PKG_VERSION)
        .author("Nymtech")
        .about("Implementation of the Nym Client")
        .subcommand(
            SubCommand::with_name("init")
                .about("Initialise a Nym client. Do this first!")
                .arg(Arg::with_name("id")
                    .long("id")
                    .help("Id of the nym-mixnet-client we want to create config for.")
                    .takes_value(true)
                    .required(true)
                )
                .arg(Arg::with_name("provider")
                    .long("provider")
                    .help("Id of the provider we have preference to connect to. If left empty, a random provider will be chosen.")
                    .takes_value(true)
                )
        )
        .subcommand(
            SubCommand::with_name("tcpsocket")
                .about("Run Nym client that listens for bytes on a TCP socket")
                .arg(
                    Arg::with_name("port")
                        .short("p")
                        .long("port")
                        .help("Port for TCP socket to listen on")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("directory")
                        .long("directory")
                        .help("Address of the directory server the client is getting topology from")
                        .takes_value(true),
                )
                .arg(Arg::with_name("id")
                    .long("id")
                    .help("Id of the nym-mixnet-client we want to run.")
                    .takes_value(true)
                    .required(true)
                )
        )
        .subcommand(
            SubCommand::with_name("websocket")
                .about("Run Nym client that listens on a websocket")
                .arg(
                    Arg::with_name("port")
                        .short("p")
                        .long("port")
                        .help("Port for websocket to listen on")
                        .takes_value(true)
                )
                .arg(
                    Arg::with_name("directory")
                        .long("directory")
                        .help("Address of the directory server the client is getting topology from")
                        .takes_value(true),
                )
                .arg(Arg::with_name("id")
                    .long("id")
                    .help("Id of the nym-mixnet-client we want to run.")
                    .takes_value(true)
                    .required(true)
                )
        )
        .get_matches();

    if let Err(e) = execute(arg_matches) {
        error!("{}", e);
        process::exit(1);
    }
}

pub mod built_info {
    // The file has been placed there by the build script.
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

fn execute(matches: ArgMatches) -> Result<(), String> {
    match matches.subcommand() {
        ("init", Some(m)) => Ok(commands::init::execute(m)),
        ("tcpsocket", Some(m)) => Ok(commands::tcpsocket::execute(m)),
        ("websocket", Some(m)) => Ok(commands::websocket::execute(m)),
        _ => Err(usage()),
    }
}

fn usage() -> String {
    banner() + "usage: --help to see available options.\n\n"
}

fn banner() -> String {
    format!(
        r#"

      _ __  _   _ _ __ ___
     | '_ \| | | | '_ \ _ \
     | | | | |_| | | | | | |
     |_| |_|\__, |_| |_| |_|
            |___/

             (client - version {:})

    "#,
        built_info::PKG_VERSION
    )
}

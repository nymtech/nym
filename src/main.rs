use clap::{App, Arg, ArgMatches, SubCommand};
use std::process;

mod clients;
mod commands;

const TCP_SOCKET_TYPE: &str = "tcp";
const WEBSOCKET_SOCKET_TYPE: &str = "websocket";

fn main() {
    let arg_matches = App::new("Nym Client")
        .version("0.1.0")
        .author("Nymtech")
        .about("Implementation of the Nym Client")
        .subcommand(
            SubCommand::with_name("init")
                .about("Initialise a Nym client")
                .arg(Arg::with_name("providerID")
                    .short("pid")
                    .long("providerID")
                    .help("Id of the provider we have preference to connect to. If left empty, a random provider will be chosen")
                    .takes_value(true)
                )
                .arg(Arg::with_name("local")
                    .short("loc")
                    .long("local")
                    .help("Flag to indicate whether the client is expected to run on the local deployment")
                )
        )
        .subcommand(
            SubCommand::with_name("run")
                .about("Run a persistent Nym client process")
                .arg(
                    Arg::with_name("customCfg")
                        .short("cfg")
                        .long("customCfg")
                        .help("Path to custom configuration file of the client")
                        .takes_value(true)
                )
        )
        .subcommand(
            SubCommand::with_name("socket")
                .about("Run a background Nym client listening on a specified socket")
                .arg(
                    Arg::with_name("customCfg")
                        .short("cfg")
                        .long("customCfg")
                        .help("Path to custom configuration file of the client")
                        .takes_value(true)
                )
                .arg(
                    Arg::with_name("socketType")
                        .short("s")
                        .long("socketType")
                        .help("Type of the socket we want to run on (tcp / websocket)")
                        .takes_value(true)
                        .required(true)
                )
                .arg(
                    Arg::with_name("port")
                        .short("p")
                        .long("port")
                        .help("Port to listen on")
                        .takes_value(true)
                        .required(true),
                )
        )
        .get_matches();

    if let Err(e) = execute(arg_matches) {
        println!("Application error: {}", e);
        process::exit(1);
    }
}

fn execute(matches: ArgMatches) -> Result<(), String> {
    match matches.subcommand() {
        ("init", Some(m)) => Ok(commands::init::execute(m)),
        ("run", Some(m)) => Ok(commands::run::execute(m)),
        ("socket", Some(m)) => Ok(commands::socket::execute(m)),

        _ => Err(String::from("Unknown command")),
    }
}
use clap::{App, Arg, ArgMatches, SubCommand};

pub mod built_info;
pub mod client;
mod commands;
pub mod config;
mod sockets;

fn main() {
    dotenv::dotenv().ok();
    pretty_env_logger::init();

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
                .arg(Arg::with_name("directory")
                     .long("directory")
                     .help("Address of the directory server the client is getting topology from")
                     .takes_value(true),
                )
                .arg(Arg::with_name("socket-type")
                    .long("socket-type")
                    .help("Type of socket to use (TCP, WebSocket or None) in all subsequent runs")
                    .takes_value(true)
                )
                .arg(Arg::with_name("port")
                    .short("p")
                    .long("port")
                    .help("Port for the socket (if applicable) to listen on in all subsequent runs")
                    .takes_value(true)
                )
        )
        .subcommand(
            SubCommand::with_name("run")
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
                .arg(Arg::with_name("provider")
                    .long("provider")
                    .help("Id of the provider we want to connect to. If overridden, it is user's responsibility to ensure prior registration happened")
                    .takes_value(true)
                )
                .arg(Arg::with_name("socket-type")
                    .long("socket-type")
                    .help("Type of socket to use (TCP, WebSocket or None)")
                    .takes_value(true)
                )
                .arg(Arg::with_name("port")
                    .short("p")
                    .long("port")
                    .help("Port for the socket (if applicable) to listen on")
                    .takes_value(true)
                )
        )
        .get_matches();

    execute(arg_matches);
}

fn execute(matches: ArgMatches) {
    match matches.subcommand() {
        ("init", Some(m)) => {
            println!("{}", banner());
            commands::init::execute(m);
        }
        ("run", Some(m)) => {
            println!("{}", banner());
            commands::run::execute(m);
        }
        _ => {
            println!("{}", usage());
        }
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

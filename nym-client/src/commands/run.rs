use crate::client::NymClient;
use crate::commands::override_config;
use crate::config::Config;
use clap::{App, Arg, ArgMatches};
use config::NymConfig;

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
}

pub fn execute(matches: &ArgMatches) {
    let id = matches.value_of("id").unwrap();

    let mut config =
        Config::load_from_file(matches.value_of("config").map(|path| path.into()), Some(id))
            .expect("Failed to load config file");

    config = override_config(config, matches);
    NymClient::new(config).run_forever();
}

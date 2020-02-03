use clap::{App, Arg, ArgMatches};
use config::NymConfig;

pub fn command_args<'a, 'b>() -> clap::App<'a, 'b> {
    App::new("init")
        .about("Initialise the mixnode")
        .arg(
            Arg::with_name("id")
                .long("id")
                .help("Id of the nym-mixnode we want to create config for.")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("layer")
                .long("layer")
                .help("The mixnet layer of this particular node")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("host")
                .long("host")
                .help("The custom host on which the mixnode will be running")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("port")
                .long("port")
                .help("The port on which the mixnode will be listening")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("announce-host")
                .long("announce-host")
                .help("The host that will be reported to the directory server")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("announce-port")
                .long("announce-port")
                .help("The port that will be reported to the directory server")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("directory")
                .long("directory")
                .help("Address of the directory server the node is sending presence and metrics to")
                .takes_value(true),
        )
}

pub fn execute(matches: &ArgMatches) {
    println!("Initialising mixnode...");

    let id = matches.value_of("id").unwrap();
    let layer = matches.value_of("layer").unwrap().parse().unwrap();
    let mut config = crate::config::Config::new(id, layer);

    // overriding config defaults here

    if let Some(host) = matches.value_of("host") {
        config = config.with_listening_host(host);
    }

    if let Some(port) = matches.value_of("port").map(|port| port.parse::<u16>()) {
        if let Err(err) = port {
            // if port was overridden, it must be parsable
            panic!("Invalid port value provided - {:?}", err);
        }
        config = config.with_listening_port(port.unwrap());
    }

    if let Some(directory) = matches.value_of("directory") {
        config = config.with_custom_directory(directory);
    }

    if let Some(announce_host) = matches.value_of("announce-host") {
        config = config.with_announce_host(announce_host);
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

    let config_save_location = config.get_config_file_save_location();
    config
        .save_to_file(None)
        .expect("Failed to save the config file");
    println!("Saved configuration file to {:?}", config_save_location);

    println!("Mixnode configuration completed.\n\n\n")
}

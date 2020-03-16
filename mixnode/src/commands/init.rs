use crate::commands::override_config;
use crate::config::persistence::pathfinder::MixNodePathfinder;
use clap::{App, Arg, ArgMatches};
use config::NymConfig;
use crypto::encryption;
use pemstore::pemstore::PemStore;

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
            Arg::with_name("location")
                .long("location")
                .help("Optional geographical location of this node")
                .takes_value(true),
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
                .help("The host on which the mixnode will be running")
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
                .help("The custom host that will be reported to the directory server")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("announce-port")
                .long("announce-port")
                .help("The custom port that will be reported to the directory server")
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
    let id = matches.value_of("id").unwrap();
    println!("Initialising mixnode {}...", id);

    let layer = matches.value_of("layer").unwrap().parse().unwrap();
    let mut config = crate::config::Config::new(id, layer);

    config = override_config(config, matches);

    let sphinx_keys = encryption::KeyPair::new();
    let pathfinder = MixNodePathfinder::new_from_config(&config);
    let pem_store = PemStore::new(pathfinder);
    pem_store
        .write_encryption_keys(sphinx_keys)
        .expect("Failed to save sphinx keys");
    println!("Saved mixnet sphinx keypair");

    let config_save_location = config.get_config_file_save_location();
    config
        .save_to_file(None)
        .expect("Failed to save the config file");
    println!("Saved configuration file to {:?}", config_save_location);

    println!("Mixnode configuration completed.\n\n\n")
}

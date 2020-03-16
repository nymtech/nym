use crate::commands::override_config;
use crate::config::persistence::pathfinder::ProviderPathfinder;
use clap::{App, Arg, ArgMatches};
use config::NymConfig;
use crypto::encryption;
use pemstore::pemstore::PemStore;

pub fn command_args<'a, 'b>() -> clap::App<'a, 'b> {
    App::new("init")
        .about("Initialise the store and forward service provider")
        .arg(
            Arg::with_name("id")
                .long("id")
                .help("Id of the sfw-provider we want to create config for.")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("location")
                .long("location")
                .help("Optional geographical location of this provider")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("mix-host")
                .long("mix-host")
                .help("The custom host on which the service provider will be running for receiving sphinx packets")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("mix-port")
                .long("mix-port")
                .help("The port on which the service provider will be listening for sphinx packets")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("clients-host")
                .long("clients-host")
                .help("The custom host on which the service provider will be running for receiving clients sfw-provider-requests")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("clients-port")
                .long("clients-port")
                .help("The port on which the service provider will be listening for clients sfw-provider-requests")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("mix-announce-host")
                .long("mix-announce-host")
                .help("The host that will be reported to the directory server")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("mix-announce-port")
                .long("mix-announce-port")
                .help("The port that will be reported to the directory server")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("clients-announce-host")
                .long("clients-announce-host")
                .help("The host that will be reported to the directory server")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("clients-announce-port")
                .long("clients-announce-port")
                .help("The port that will be reported to the directory server")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("inboxes")
                .long("inboxes")
                .help("Directory with inboxes where all packets for the clients are stored")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("clients-ledger")
                .long("clients-ledger")
                .help("[UNIMPLEMENTED] Ledger file containing registered clients")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("directory")
                .long("directory")
                .help("Address of the directory server the node is sending presence data to")
                .takes_value(true),
        )
}

pub fn execute(matches: &ArgMatches) {
    let id = matches.value_of("id").unwrap();
    println!("Initialising sfw service provider {}...", id);

    let mut config = crate::config::Config::new(id);

    config = override_config(config, matches);

    let sphinx_keys = encryption::KeyPair::new();
    let pathfinder = ProviderPathfinder::new_from_config(&config);
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

    println!("Service provider configuration completed.\n\n\n")
}

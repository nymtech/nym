use crate::config::persistance::pathfinder::ClientPathfinder;
use crate::config::SocketType;
use clap::ArgMatches;
use config::NymConfig;
use crypto::identity::MixIdentityKeyPair;
use pemstore::pemstore::PemStore;

pub fn execute(matches: &ArgMatches) {
    println!("Initialising client...");

    let id = matches.value_of("id").unwrap(); // required for now
    let mut config = crate::config::Config::new(id);

    if let Some(directory) = matches.value_of("directory") {
        config = config.with_custom_directory(directory);
    }

    if let Some(provider_id) = matches.value_of("provider") {
        config = config.with_provider_id(provider_id);
    }

    if let Some(socket_type) = matches.value_of("socket-type") {
        config = config.with_socket(SocketType::from_string(socket_type));
    }

    if let Some(port) = matches.value_of("port").map(|port| port.parse::<u16>()) {
        if let Err(err) = port {
            // if port was overridden, it must be parsable
            panic!("Invalid port value provided - {:?}", err);
        }
        config = config.with_port(port.unwrap());
    }

    let mix_identity_keys = MixIdentityKeyPair::new();
    let pathfinder = ClientPathfinder::new_from_config(&config);

    let pem_store = PemStore::new(pathfinder);
    pem_store
        .write_identity(mix_identity_keys)
        .expect("Failed to save identity keys");
    println!("Saved mixnet identity keypair");

    // TODO: perform provider registration here

    let config_save_location = config.get_config_file_save_location();
    config
        .save_to_file(None)
        .expect("Failed to save the config file");
    println!("Saved configuration file to {:?}", config_save_location);
    println!("Client configuration completed.\n\n\n")
}

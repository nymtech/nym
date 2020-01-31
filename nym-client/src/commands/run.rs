use crate::client::NymClient;
use crate::config::persistance::pathfinder::ClientPathfinder;
use crate::config::{Config, SocketType};
use clap::ArgMatches;
use config::NymConfig;
use pemstore::pemstore::PemStore;

pub fn execute(matches: &ArgMatches) {
    let id = matches.value_of("id").unwrap();

    let mut config_file =
        Config::load_from_file(matches.value_of("config").map(|path| path.into()), Some(id))
            .expect("Failed to load config file");

    if let Some(directory) = matches.value_of("directory") {
        config_file = config_file.with_custom_directory(directory);
    }

    if let Some(provider_id) = matches.value_of("provider") {
        config_file = config_file.with_provider_id(provider_id);
    }

    if let Some(socket_type) = matches.value_of("socket-type") {
        config_file = config_file.with_socket(SocketType::from_string(socket_type));
    }

    if let Some(port) = matches.value_of("port").map(|port| port.parse::<u16>()) {
        if let Err(err) = port {
            // if port was overridden, it must be parsable
            panic!("Invalid port value provided - {:?}", err);
        }
        config_file = config_file.with_port(port.unwrap());
    }

    let identity_keypair = PemStore::new(ClientPathfinder::new_from_config(&config_file))
        .read_identity()
        .expect("Failed to read stored identity key files");

    println!(
        "Public key: {}",
        identity_keypair.public_key.to_base58_string()
    );

    let client = NymClient::new(config_file);
    //
    //    client.start().unwrap();
}

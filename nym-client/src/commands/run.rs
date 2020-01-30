use crate::client::{NymClient, SocketType};
use crate::config::persistance::pathfinder::ClientPathfinder;
use crate::config::Config;
use clap::ArgMatches;
use config::NymConfig;
use crypto::identity::MixIdentityKeyPair;
use pemstore::pemstore::PemStore;
use std::net::ToSocketAddrs;

pub fn execute(matches: &ArgMatches) {
    let id = matches.value_of("id").unwrap();

    let mut config_file =
        Config::load_from_file(matches.value_of("config").map(|path| path.into()), Some(id))
            .expect("Failed to load config file");

    if let Some(directory) = matches.value_of("directory") {
        config_file = config_file.with_custom_directory(directory.to_string());
    }

    if let Some(provider_id) = matches.value_of("provider") {
        config_file = config_file.with_provider_id(provider_id.to_string());
    }

    if let Some(socket_type) = matches.value_of("socket-type") {
        config_file = config_file.with_socket(socket_type.into());
    }

    if let Some(port) = matches.value_of("port").map(|port| port.parse::<u16>()) {
        if let Err(err) = port {
            panic!("Invalid port value provided - {:?}", err);
        }
        config_file = config_file.with_port(port.unwrap());
    }

    println!("Going to use the following config: {:#?}", config_file);

    //
    //    let socket_address = ("127.0.0.1", port)
    //        .to_socket_addrs()
    //        .expect("Failed to combine host and port")
    //        .next()
    //        .expect("Failed to extract the socket address from the iterator");
    //
    //    // TODO: currently we know we are reading the 'DummyMixIdentityKeyPair', but how to properly assert the type?
    //    let keypair: MixIdentityKeyPair = PemStore::new(ClientPathfinder::new(id))
    //        .read_identity()
    //        .unwrap();
    //
    //    // TODO: reading auth_token from disk (if exists);
    //
    //    println!("Public key: {}", keypair.public_key.to_base58_string());
    //
    //    let auth_token = None;
    //    let client = NymClient::new(
    //        keypair,
    //        socket_address,
    //        directory_server,
    //        auth_token,
    //        SocketType::WebSocket,
    //    );
    //
    //    client.start().unwrap();
}

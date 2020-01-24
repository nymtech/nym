use crate::client::{NymClient, SocketType};
use crate::config::persistance::pathfinder::ClientPathfinder;
use clap::ArgMatches;
use crypto::identity::{DummyMixIdentityKeyPair, MixnetIdentityKeyPair, MixnetIdentityPublicKey};
use pemstore::pemstore::PemStore;
use std::net::ToSocketAddrs;

pub fn execute(matches: &ArgMatches) {
    let id = matches.value_of("id").unwrap().to_string();
    let port = match matches.value_of("port").unwrap_or("9001").parse::<u16>() {
        Ok(n) => n,
        Err(err) => panic!("Invalid port value provided - {:?}", err),
    };

    let directory_server = matches
        .value_of("directory")
        .unwrap_or("https://directory.nymtech.net")
        .to_string();

    println!("Starting TCP socket on port: {:?}", port);
    println!("Listening for messages...");

    let socket_address = ("127.0.0.1", port)
        .to_socket_addrs()
        .expect("Failed to combine host and port")
        .next()
        .expect("Failed to extract the socket address from the iterator");

    // TODO: currently we know we are reading the 'DummyMixIdentityKeyPair', but how to properly assert the type?
    let keypair: DummyMixIdentityKeyPair = PemStore::new(ClientPathfinder::new(id))
        .read_identity()
        .unwrap();
    // TODO: reading auth_token from disk (if exists);

    println!("Public key: {}", keypair.public_key.to_b64_string());

    let mut temporary_address = [0u8; 32];
    let public_key_bytes = keypair.public_key().to_bytes();
    temporary_address.copy_from_slice(&public_key_bytes[..]);
    let auth_token = None;
    let client = NymClient::new(
        temporary_address,
        socket_address.clone(),
        directory_server,
        auth_token,
        SocketType::TCP,
    );

    client.start().unwrap();
}

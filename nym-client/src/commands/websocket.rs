use crate::banner;
use crate::clients::{NymClient, SocketType};
use crate::persistence::pemstore;

use clap::ArgMatches;
use crypto::identity::{MixnetIdentityKeyPair, MixnetIdentityPublicKey};
use std::net::ToSocketAddrs;

pub fn execute(matches: &ArgMatches) {
    println!("{}", banner());

    let id = matches.value_of("id").unwrap().to_string();
    let port = match matches.value_of("port").unwrap_or("9001").parse::<u16>() {
        Ok(n) => n,
        Err(err) => panic!("Invalid port value provided - {:?}", err),
    };

    let directory_server = matches
        .value_of("directory")
        .unwrap_or("https://directory.nymtech.net")
        .to_string();

    println!("Starting websocket on port: {:?}", port);
    println!("Listening for messages...");

    let socket_address = ("127.0.0.1", port)
        .to_socket_addrs()
        .expect("Failed to combine host and port")
        .next()
        .expect("Failed to extract the socket address from the iterator");

    let keypair = pemstore::read_mix_identity_keypair_from_disk(id);
    // TODO: reading auth_token from disk (if exists);

    println!("Public key: {}", keypair.public_key.to_b64_string());

    let mut temporary_address = [0u8; 32];
    let public_key_bytes = keypair.public_key().to_bytes();
    temporary_address.copy_from_slice(&public_key_bytes[..]);
    let auth_token = None;
    let client = NymClient::new(
        temporary_address,
        socket_address,
        directory_server,
        auth_token,
        SocketType::WebSocket,
    );

    client.start().unwrap();
}

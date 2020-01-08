use crate::banner;
use crate::clients::{NymClient, SocketType};
use crate::persistence::pemstore;

use clap::ArgMatches;
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

    println!("Starting TCP socket on port: {:?}", port);
    println!("Listening for messages...");

    let socket_address = ("127.0.0.1", port)
        .to_socket_addrs()
        .expect("Failed to combine host and port")
        .next()
        .expect("Failed to extract the socket address from the iterator");

    let keypair = pemstore::read_keypair_from_disk(id);
    let auth_token = None;

    let client = NymClient::new(
        keypair.public_bytes(),
        socket_address.clone(),
        directory_server,
        auth_token,
        SocketType::TCP,
    );

    client.start().unwrap();
}

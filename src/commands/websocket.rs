use crate::banner;
use crate::clients::NymClient;
use crate::persistence::pemstore;
use crate::sockets::ws;

use clap::ArgMatches;
use std::net::ToSocketAddrs;

pub fn execute(matches: &ArgMatches) {
    println!("{}", banner());

    let is_local = matches.is_present("local");
    let id = matches.value_of("id").unwrap().to_string();
    let port = match matches.value_of("port").unwrap_or("9001").parse::<u16>() {
        Ok(n) => n,
        Err(err) => panic!("Invalid port value provided - {:?}", err),
    };

    println!("Starting websocket on port: {:?}", port);
    println!("Listening for messages...");

    let socket_address = ("127.0.0.1", port)
        .to_socket_addrs()
        .expect("Failed to combine host and port")
        .next()
        .expect("Failed to extract the socket address from the iterator");


    let is_local = matches.is_present("local");
    println!("Starting client, local: {:?}", is_local);

    let keypair = pemstore::read_keypair_from_disk(id);
    let client = NymClient::new(keypair.public_bytes(), is_local);

    client.start(socket_address).unwrap();


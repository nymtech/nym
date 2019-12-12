use crate::banner;
use crate::node::MixNode;
use clap::ArgMatches;

use curve25519_dalek::scalar::Scalar;
use std::net::ToSocketAddrs;

pub fn start(matches: &ArgMatches) {
    println!("{}", banner());
    println!("Starting mixnode...");

    let host = matches.value_of("host").unwrap_or("0.0.0.0");

    let port = match matches.value_of("port").unwrap().parse::<u16>() {
        Ok(n) => n,
        Err(err) => panic!("Invalid port value provided - {:?}", err),
    };

    let layer = match matches.value_of("layer").unwrap().parse::<usize>() {
        Ok(n) => n,
        Err(err) => panic!("Invalid layer value provided - {:?}", err),
    };

    let secret_key: Scalar = match matches.value_of("keyfile") {
        Some(keyfile) => {
            println!("TODO: load keyfile from <{:?}>", keyfile);
            Default::default()
        }
        None => {
            println!("TODO: generate fresh sphinx keypair");
            Default::default()
        }
    };

    let socket_address = (host, port)
        .to_socket_addrs()
        .expect("Failed to combine host and port")
        .next()
        .expect("Failed to extract the socket address from the iterator");

    println!("Startup complete on: {}", socket_address);
    println!("Listening for incoming packets...");

    // make sure our socket_address is equal to our predefined-hardcoded value
    // assert_eq!("127.0.0.1:8080", socket_address.to_string());

    let mix = MixNode::new(socket_address, secret_key, layer);
    mix.start_listening().unwrap();
}

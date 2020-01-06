use crate::banner;
use crate::node;
use crate::node::presence;
use crate::node::MixNode;
use clap::ArgMatches;

use std::net::ToSocketAddrs;
use std::thread;

pub fn start(matches: &ArgMatches) {
    println!("{}", banner());
    println!("Starting mixnode...");

    let config = new_config(matches);
    println!("Public key: {}", config.public_key_string());

    println!(
        "Listening for incoming packets on {}",
        config.socket_address
    );

    let mix = MixNode::new(&config);
    thread::spawn(move || {
        let notifier = presence::Notifier::new(&config);
        notifier.run();
    });
    mix.start().unwrap();
}

fn new_config(matches: &ArgMatches) -> node::Config {
    let host = matches.value_of("host").unwrap();

    let port = match matches.value_of("port").unwrap_or("1789").parse::<u16>() {
        Ok(n) => n,
        Err(err) => panic!("Invalid port value provided - {:?}", err),
    };

    let layer = match matches.value_of("layer").unwrap().parse::<usize>() {
        Ok(n) => n,
        Err(err) => panic!("Invalid layer value provided - {:?}", err),
    };

    let socket_address = (host, port)
        .to_socket_addrs()
        .expect("Failed to combine host and port")
        .next()
        .expect("Failed to extract the socket address from the iterator");

    let (secret_key, public_key) = sphinx::crypto::keygen();

    let directory_server = matches
        .value_of("directory")
        .unwrap_or("https://directory.nymtech.net")
        .to_string();

    node::Config {
        directory_server,
        layer,
        public_key,
        socket_address,
        secret_key,
    }
}

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

    // println!("Startup on: {}", config.socket_address);
    println!("Listening for incoming packets...");

    let mix = MixNode::new(&config);
    thread::spawn(move || {
        let notifier = presence::Notifier::new(&config);
        notifier.run();
    });
    mix.start().unwrap();
}

fn new_config(matches: &ArgMatches) -> node::Config {
    let host = matches.value_of("host").unwrap_or("0.0.0.0");

    let port = match matches.value_of("port").unwrap_or("1789").parse::<u16>() {
        Ok(n) => n,
        Err(err) => panic!("Invalid port value provided - {:?}", err),
    };

    let layer = match matches.value_of("layer").unwrap().parse::<usize>() {
        Ok(n) => n,
        Err(err) => panic!("Invalid layer value provided - {:?}", err),
    };

    let is_local = matches.is_present("local");

    let socket_address = (host, port)
        .to_socket_addrs()
        .expect("Failed to combine host and port")
        .next()
        .expect("Failed to extract the socket address from the iterator");

    let (secret_key, public_key) = sphinx::crypto::keygen();

    let directory_server = if is_local {
        "http://localhost:8080".to_string()
    } else {
        "https://directory.nymtech.net".to_string()
    };

    node::Config {
        directory_server,
        layer,
        public_key,
        socket_address,
        secret_key,
    }
}
